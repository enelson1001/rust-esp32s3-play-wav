use std::{convert::TryInto, time::Instant};

use {
    embedded_sdmmc::{SdCard, TimeSource, Timestamp},
    esp_idf_hal::{
        delay::{Ets, FreeRtos, TickType},
        gpio::*,
        i2s::{
            config::{
                Config, DataBitWidth, SlotMode, StdClkConfig, StdConfig, StdGpioConfig,
                StdSlotConfig,
            },
            I2sDriver, I2sTx,
        },
        prelude::*,
        spi::{config::Duplex, Dma, SpiConfig, SpiDeviceDriver, SpiDriver, SpiDriverConfig},
        units::FromValueType,
    },
    log::*,
};

pub struct SdMmcClock;

impl TimeSource for SdMmcClock {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

// The filename needs to correspond to 8.3 naming, so it should be no more than 8 characters long, with the extension .wav at the end.
const WAV_FILE: &str = "gettys_m.wav";
//const FILE_TO_READ: &str = "laugh_m.wav";

const BLOCK_TIME: TickType = TickType::new(100_000_000); // Long enough we should not expect to ever return.
const SAMPLE_RATE_HZ: u32 = 44100;
const BYTES_IN_HEADER: u8 = 44;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("========== Starting App ==========");

    let peripherals = Peripherals::take()?;

    //============================================================================================================
    //                             Create the SPI device driver for SDCard
    //============================================================================================================
    info!("========== Creating SPI driver ==========");
    let spi_driver_config = SpiDriverConfig::new().dma(Dma::Auto(4096));
    let spi_config = SpiConfig::new()
        .duplex(Duplex::Full)
        .baudrate(24.MHz().into());

    let spi = SpiDeviceDriver::new(
        SpiDriver::new(
            peripherals.spi2,
            peripherals.pins.gpio12,       // SCK
            peripherals.pins.gpio11,       // MOSI
            Some(peripherals.pins.gpio13), // MISO
            &spi_driver_config,
        )?,
        Option::<AnyOutputPin>::None, // CS - don't use here
        &spi_config,
    )?;

    //============================================================================================================
    //                                  Create the I2S driver
    // auto_clear = false so that the 2 dma buffers are a ring buffer
    // dma_bufffer_count = 2 , minimum dma buffer count
    // frames_per_buffer = 512 frames; is 1024 bytes for mono or 2048 bytes for stereo
    // bits per sample = 16 bits = 2 bytes
    // data bytes for a mono frame = 2 bytes
    // data bytes for a stereo frame = 4 bytes
    // total allocated storage for mono wav = 2048 bytes == 512 frames x 2 data bytes per mono frame x 2 dma buffers
    //
    // philips_slot_default =
    //      number of slots = 2 slots (one left channel slot, one right channel slot),
    //      sample size = 16 bits,
    //      slotMode = Mono
    //
    // one frame = one lrclk cycle or ws cycle
    // one frame for a 16 bits per sample = 32 bits; 16 bits for left channel and 16 bits per right channel = 4 bytes
    // one frame for mono = transmit data on left channel, the right channel is 0 for all 16 bits
    // one frame for stereo = transmit data on both channels
    //
    // For Philips Format
    // mono = left channel (left slot) has data, right channel (right slot has all 0 for 16 bits)
    // stereo = left channels (left slot) has data, right channel (right slot) has data
    //
    // With sample rate = 44.1KHz
    // time to send one frame (one lrclk cycle) = 1 / 44.1KHz = 22.675 micro seconds
    // time to send 512 frames = 512 x 22.675 microseconds = 11.6 milliseconds
    //
    // Note from the MAX98357A data sheet:
    // LRCLK ONLY supports 8kHz, 16kHz, 32kHz, 44.1kHz, 48kHz, 88.2kHz and 96kHz frequencies.
    // LRCLK clocks at 11.025kHz, 12kHz, 22.05kHz and 24kHz are NOT supported.
    //
    // So SAMPLE_RATE_HZ must be one of the following: 8kHz, 16kHz, 32kHz, 44.1kHz, 48kHz, 88.2kHz, 96kHz
    //============================================================================================================
    info!("========== Creating I2S driver ==========");
    let i2s_config = StdConfig::new(
        Config::default()
            .auto_clear(false)
            .dma_buffer_count(2)
            .frames_per_buffer(512),
        StdClkConfig::from_sample_rate_hz(SAMPLE_RATE_HZ),
        StdSlotConfig::philips_slot_default(DataBitWidth::Bits16, SlotMode::Mono),
        StdGpioConfig::default(),
    );

    let i2s_0 = peripherals.i2s0;
    let bclk = peripherals.pins.gpio0; // version 1.1, version 1.0 uses gpio19
    let dout = peripherals.pins.gpio17;
    let ws = peripherals.pins.gpio18; // same as lrclk
    let mclk = AnyIOPin::none();

    let mut i2s = I2sDriver::<I2sTx>::new_std_tx(i2s_0, &i2s_config, bclk, dout, mclk, ws)?;

    //============================================================================================================
    //                      Create the SD Card Interface using SPI device driver
    //============================================================================================================
    info!("========== Creating SD Card interface ==========");
    let sdcard_cs = PinDriver::output(peripherals.pins.gpio10)?;
    let sdcard = SdCard::new(spi, sdcard_cs, Ets);

    // initialize SD Card
    sdcard
        .num_bytes()
        .map_err(|e| anyhow::anyhow!("SdCard error: {:?}", e))?;

    let mut volume_mgr = embedded_sdmmc::VolumeManager::new(sdcard, SdMmcClock);
    let volume = volume_mgr
        .open_volume(embedded_sdmmc::VolumeIdx(0))
        .map_err(|e| anyhow::anyhow!("SdCard error: {:?}", e))?;

    // Open the root directory.
    let root_dir = volume_mgr
        .open_root_dir(volume)
        .map_err(|e| anyhow::anyhow!("SdCard error: {:?}", e))?;

    // Open the "WAV_FILE" located in the root directory
    info!("========== Opening wav file ==========");
    let wav_file = volume_mgr
        .open_file_in_dir(root_dir, WAV_FILE, embedded_sdmmc::Mode::ReadOnly)
        .map_err(|e| anyhow::anyhow!("SdCard error: {:?}", e))?;

    //============================================================================================================
    //                              Read header from WAV file
    //============================================================================================================
    info!("========== Reading header from WAV file ==========");
    let mut header = [0u8; BYTES_IN_HEADER as usize];
    volume_mgr
        .read(wav_file, &mut header)
        .expect("read header from wav file");
    let riff_id = std::str::from_utf8(&header[0..4]).unwrap();
    let file_size = u32::from_le_bytes(header[4..8].try_into().unwrap());
    let file_type = std::str::from_utf8(&header[8..12]).unwrap();
    let chunk_format = std::str::from_utf8(&header[12..16]).unwrap();
    let size_of_format_section = u32::from_le_bytes(header[16..20].try_into().unwrap());
    let format = u16::from_le_bytes(header[20..22].try_into().unwrap());
    let num_of_channels = u16::from_le_bytes(header[22..24].try_into().unwrap());
    let sampling_rate = u32::from_le_bytes(header[24..28].try_into().unwrap());
    let byte_rate = u32::from_le_bytes(header[28..32].try_into().unwrap());
    let block_align = u16::from_le_bytes(header[32..34].try_into().unwrap());
    let bits_per_sample = u16::from_le_bytes(header[34..36].try_into().unwrap());
    let data_section_id = std::str::from_utf8(&header[36..40]).unwrap();
    let size_of_data = u32::from_le_bytes(header[40..44].try_into().unwrap());

    warn!("========== Header Info ==========");
    warn!("riff ID = {:?}", riff_id);
    warn!("file size minus 8 bytes = {:?}", file_size);
    warn!("RIFF format = {:?}", file_type);
    warn!("chunk format ID = {:?}", chunk_format);
    warn!("size of format section - 8 = {:?}", size_of_format_section);
    warn!("format = {:?}", format);
    warn!(
        "number of channels (1=mono, 2=stereo) = {:?}",
        num_of_channels
    );
    warn!("sampling rate = {:?}", sampling_rate);
    warn!("byte rate = {:?}", byte_rate);
    warn!("block align = {:?}", block_align);
    warn!("bits per sample = {:?}", bits_per_sample);
    warn!("data section id = {:?}", data_section_id);
    warn!("data size in bytes = {:?}", size_of_data);

    let mut fake_buffer = [0u8; 1024];

    // Set file index to the end of the header section
    volume_mgr
        .file_seek_from_start(wav_file, BYTES_IN_HEADER as u32)
        .expect("failed to seek");

    let mut now = Instant::now();
    let _bytes_read = volume_mgr.read(wav_file, &mut fake_buffer).expect("read");
    let new_now = Instant::now();
    info!(
        "========== Time to read 1024 bytes {:?} ==========",
        new_now.duration_since(now)
    );

    //============================================================================================================
    //                                      Play WAV file
    //============================================================================================================
    info!("========== Started playing {:?} file ==========", WAV_FILE);

    // CHUNK_SIZE = 1024 bytes == 512 frames where the left slot contains data from buffer (2 bytes),
    // and the right slot (2 bytes) is automatically set to zero because wav is mono
    const CHUNK_SIZE: usize = 1024;
    let mut buffer = [0u8; CHUNK_SIZE];
    let mut bytes_read: usize;
    let mut data_read: usize = 0;

    // Reset the file index to the end of header section
    volume_mgr
        .file_seek_from_start(wav_file, BYTES_IN_HEADER as u32)
        .map_err(|e| anyhow::anyhow!("SdCard error: {:?}", e))?;

    now = Instant::now();
    i2s.tx_enable().unwrap();

    while data_read < (size_of_data) as usize {
        bytes_read = volume_mgr
            .read(wav_file, &mut buffer)
            .map_err(|e| anyhow::anyhow!("SdCard error: {:?}", e))?;
        data_read += bytes_read;

        i2s.write_all(&buffer[..bytes_read], BLOCK_TIME.into())
            .map_err(|e| anyhow::anyhow!("I2S error: {:?}", e))?;
    }

    i2s.tx_disable().unwrap();

    info!(
        "========== Finsihed playing WAV file, took {:?} seconds to play ==========",
        Instant::now().duration_since(now)
    );

    FreeRtos::delay_ms(5000);

    info!("========== Goodbye ==========");

    Ok(())
}
