# Rust ESP32S3 Play Wav

The purpose of this demo is to play a WAV file that is located on the SDCard.

## Development Board and Parts
Aliexpress ESP32-8048S070 - 7 inch 800x400 TN RGB with ESP32S3, 8M PSRAM, 16M Flash, 512KB SRAM

Adafruit Mini Oval Speaker - 8 ohm 1 Watt


## Overview
The program will play either the laugh_m.wav or gettys_m.wav file located on SDCard.  The program uses the I2S interface to the MAX98357A PCM Class D amplifier.

## Comments
For Aliexpress ESP32-8048S070C development board REV 1.1 uses GPIO0 for BCLK, REV 1.0 use GPIO19 for BCLK.

The wiring for the speaker at the connector did not match the connector on the development board. The positive and negative of the speaker were reversed but the sound seemed OK.

Since we only have a single MAX98357A chip we can only play mono wav files.

With the speaker I used the sound quality and sound volume was OK but a better speaker may produce better sound quality.

I enabled I2S debugging in the sdkconfig.defaults

## Sample of terminal output
```
I (423) rust_esp32s3_play_wav: ========== Starting App ==========
I (433) rust_esp32s3_play_wav: ========== Creating SPI driver ==========
I (443) rust_esp32s3_play_wav: ========== Creating I2S driver ==========
D (453) i2s_common: tx channel is registered on I2S0 successfully
D (453) i2s_common: DMA malloc info: dma_desc_num = 2, dma_desc_buf_size = dma_frame_num * slot_num * data_bit_width = 1024
D (463) i2s_std: Clock division info: [sclk] 160000000 Hz [mdiv] 14 [mclk] 11289600 Hz [bdiv] 8 [bclk] 1411200 Hz
D (473) i2s_std: The tx channel on I2S0 has been initialized to STD mode successfully
I (483) rust_esp32s3_play_wav: ========== Creating SD Card interface ==========
I (493) gpio: GPIO[10]| InputEn: 0| OutputEn: 0| OpenDrain: 0| Pullup: 1| Pulldown: 0| Intr:0 
I (503) rust_esp32s3_play_wav: ========== Opening wav file ==========
I (513) rust_esp32s3_play_wav: ========== Reading header from WAV file ==========
W (523) rust_esp32s3_play_wav: ========== Header Info ==========
W (523) rust_esp32s3_play_wav: riff ID = "RIFF"
W (533) rust_esp32s3_play_wav: file size minus 8 bytes = 882308
W (533) rust_esp32s3_play_wav: RIFF format = "WAVE"
W (543) rust_esp32s3_play_wav: chunk format ID = "fmt "
W (543) rust_esp32s3_play_wav: size of format section - 8 = 16
W (553) rust_esp32s3_play_wav: format = 1
W (553) rust_esp32s3_play_wav: number of channels (1=mono, 2=stereo) = 1
W (563) rust_esp32s3_play_wav: sampling rate = 44100
W (573) rust_esp32s3_play_wav: byte rate = 88200
W (573) rust_esp32s3_play_wav: block align = 2
W (583) rust_esp32s3_play_wav: bits per sample = 16
W (583) rust_esp32s3_play_wav: data section id = "data"
W (593) rust_esp32s3_play_wav: data size in bytes = 882272
I (603) rust_esp32s3_play_wav: ========== Time to read 1024 bytes 3.205ms ==========
I (603) rust_esp32s3_play_wav: ========== Started playing "gettys_m.wav" file ==========
D (613) i2s_common: i2s tx channel enabled
D (10623) i2s_common: i2s tx channel disabled
I (10623) rust_esp32s3_play_wav: ========== Finsihed playing WAV file, took 10.008226s seconds to play ==========
I (15633) rust_esp32s3_play_wav: ========== Goodbye ==========
I (15633) gpio: GPIO[11]| InputEn: 0| OutputEn: 0| OpenDrain: 0| Pullup: 1| Pulldown: 0| Intr:0 
I (15633) gpio: GPIO[13]| InputEn: 0| OutputEn: 0| OpenDrain: 0| Pullup: 1| Pulldown: 0| Intr:0 
I (15643) gpio: GPIO[12]| InputEn: 0| OutputEn: 0| OpenDrain: 0| Pullup: 1| Pulldown: 0| Intr:0 
I (15653) gpio: GPIO[10]| InputEn: 0| OutputEn: 0| OpenDrain: 0| Pullup: 1| Pulldown: 0| Intr:0 
E (15663) i2s_common: i2s_channel_disable(1030): the channel has not been enabled yet
D (15673) i2s_common: tx channel on I2S0 deleted
I (15673) main_task: Returned from app_main()
```

## Flashing the ESP32S3 device
I used the following command to flash the ESP32S3 device.
```
$ cargo espflash flash --partition-table=partition-table/partitions.csv --monitor
```

## Picture of Aliexpress ESP32S3 with speaker attached
![esp32s3-sound](photos/sound.jpg)


# Versions
### v1.0 : 
- initial release
