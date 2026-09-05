# Native Instruments Maschine Mikro MK3 — HID protocol

Reverse-engineered notes from the MikroDeck project. Everything here was verified
against real hardware on Windows 11, with a webcam pointed at the device and
numeric brightness measurement to avoid fooling ourselves.

The device does **not** send MIDI over USB. It is a pure HID device; the "MIDI
mode" you see in Native Instruments software is emulated by their driver. Talking
HID directly gives you full access: pad LEDs, button LEDs, the touch strip, the
screen, pad pressure, buttons and the encoder — with no NI driver involved.

## Device identity

| Field | Value |
| --- | --- |
| Vendor ID | `0x17CC` |
| Product ID | `0x1700` |
| Interface | `MI_00` |
| Endpoints | `0x81` IN, `0x01` OUT |
| Driver | the plain Windows HID stack — no WinUSB, no Zadig, no driver swap |

Hardware: one 128x32 monochrome screen, one push encoder, 16 RGB pads, a touch
strip with 25 blue LEDs, and buttons with their own LEDs.

## The one prerequisite (Windows)

Out of the box the device accepts reads but ignores LED writes.

**Running Maschine 2 once, as administrator, with the device connected, writes
permanent state into the device.** After that it accepts LED commands from any
process, at any time, with no initialization sequence at all. The state survives
unplugging the cable and rebooting the machine.

This was proven with the cable physically reconnected and nothing from NI
running: a raw write of report `0x80`, with no reads, 9 ms after opening the
device, lit everything up. The screen stayed dark, which proves no NI software
took part.

We never found the specific command that flips this bit. Hooking Maschine 2's
calls (API Monitor or similar) to find it is the obvious next step for anyone who
wants to remove the prerequisite.

### Five theories that turned out to be wrong

Recorded so nobody burns time on them again:

1. Windows was inflating the packet to 265 bytes and the firmware discarded it.
2. "MIDI mode" was what enabled the LEDs.
3. A specific initialization sequence had to be replayed on every connect.
4. There was a time window after connection during which writes were accepted.
5. There was a race between opening the device and the first write.

None of them. It is the one-time administrator run.

## Reading: input reports

### Report `0x01` — buttons, encoder, touch strip

Sent whenever any button, the encoder or the touch strip changes.

### Report `0x02` — pads

Pads report pressure, not just on/off, with a range of **0 to 4095**. This is what
makes a light-press/firm-press distinction possible.

> **Read-buffer gotcha:** allocate at least 256 bytes for the read buffer. A
> 64-byte buffer silently drops the larger packets, with no error, and looks like
> "the device stopped sending events".

## Writing: LEDs, report `0x80`

81 bytes total:

| Offset | Length | Meaning |
| --- | --- | --- |
| 0 | 1 | report ID (`0x80`) |
| 1..=39 | 39 | button LEDs |
| 40..=55 | 16 | pad LEDs |
| 56..=80 | 25 | touch strip LEDs |

### Pad and strip bytes

```
byte = (colour << 2) | brightness
```

- `colour` is 1..=17 (see table below); `0` means off.
- `brightness` is 0..=3. **Brightness 0 is the dim level, not off.** What turns a
  pad off is writing the whole byte as zero.

Colour indices, in order: red, orange, light orange, warm yellow, yellow, lime,
green, mint, cyan, turquoise, blue, plum, violet, purple, magenta, fuchsia, white.

### Button bytes

Buttons are monochrome (each has a fixed colour in hardware — PLAY is green, REC
is red, most are white). Only brightness matters:

| Value | Meaning |
| --- | --- |
| `0x00` | off |
| `0x7C` | dim |
| `0x7E` | normal |
| `0x7F` | bright |

### Pad numbering

The raw index in the report does not match the numbers printed on the device.
Printed pad 13 is the top-left corner and printed pad 1 is the bottom-left. The
mapping from printed order to raw index is:

```
[12, 13, 14, 15,
  8,  9, 10, 11,
  4,  5,  6,  7,
  0,  1,  2,  3]
```

## Writing: the screen, report `0xE0`

- 128 x 32 pixels, 1 bit per pixel.
- Sent as **two packets of 265 bytes**, each covering half the screen
  (128 columns x 16 rows).
- **The bitmap is inverted: bit 1 turns the pixel OFF.**
- `y` is expressed in blocks of 8 rows. Within a byte, row 0 is the least
  significant bit.

Packet header, then 256 bytes of pixel data:

| Offset | Bytes | Meaning |
| --- | --- | --- |
| 0 | 1 | report ID (`0xE0`) |
| 1..2 | 2 | x, little endian |
| 3..4 | 2 | y, in blocks of 8 rows |
| 5..6 | 2 | width |
| 7..8 | 2 | height, in blocks of 8 rows |
| 9..264 | 256 | pixel data |

You can address sub-regions, but the USB transfer still has to be sent at the
full size.

## Write-rate limit

**The device accepts roughly 31 writes per second.** Going over that corrupts or
drops packets — you see torn screen halves and LED frames that do not land.

Budget accordingly. In MikroDeck the write thread sends at most one packet per
tick, prioritising the LED frame, and puts 12 ms between the two screen halves.
A full screen redraw therefore costs two writes, which caps smooth screen
animation at about 8 frames per second in practice.

Send the LED frame only when a byte actually changed. Dirty-tracking the frame is
what keeps the budget usable.

## Verifying LED work without fooling yourself

Ambient light and webcam auto-exposure will happily make an unlit pad look lit.
Two things made the difference in this project:

1. A webcam pointed at the device with scripted photo capture.
2. Numeric per-region brightness measurement, normalised against a fixed patch of
   the wall in frame, to cancel out exposure drift.

Without the numeric step, exposure variation reads as "the LED turned on".

## Reference implementations

- [pymikro](https://github.com/flokapi/pymikro) — Python, this exact device. The
  best map of the packet layouts.
- [ktemkin's MK3 notes](https://gist.github.com/ktemkin/89253ecf10c5078f47607776564de83b)
  — reverse-engineering notes for the MK3 family.
- MikroDeck's own `motor/src/hid/` — Rust, written from the above plus hardware
  testing.
