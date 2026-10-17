pen-steer
=========

<img src="assets/icon.svg"  alt="pen-steer app icon" height="196px">

Use a drawing tablet or similar as a virtual steering wheel.

It can be used for video games like
[Euro Truck Simulator 2](https://eurotrucksimulator2.com/) and
[BeamNG.drive](https://beamng.com/game/), or in other applications involving a
steering-wheel-type input method (but why?).

Pen-steer supports Linux and Windows only.

# Very much a work in progress.

This partial rewrite currently has even less features than before the rewrite.
This will eventually change.

![screenshot](assets/screenshot.png)

*Above is a screenshot of what the app looks like. Only some of the
functionality is actually funcitonal right now.*

## Why?

Part of the inspiration is this bit in a
[Youtube video](https://youtu.be/fNZ1F7jTSMA?t=34) (at 00:34) which shows a
tank cannon (among other parts) being controlled with a DIY setup.

Some games like ETS2 offer mouse steering, which is alright (and in my opinion,
quite superior to keyboard steering if you bind extra keys to look left and
right), but I wanted to see if it would be practical to use a drawing tablet.
Surprisingly, it's not that bad; it does take a bit of practice to draw a circle
at the centre. I would still use an actual wheel if I could afford one, though.

As far as I know, there isn't anything else like this (or I didn't search
enough), so I made this. Though, I am probably not the first person to try
something like this.

This Rust application is not my first attempt, I wrote the initial prototype in
C and a cursed bunch of `ioctl()`s.

## Quick Usage Guide

To turn the steering wheel, drag the pen in a big
[circle](https://youtu.be/xVUQvceUnUg) around the middle of the tablet area.
Try to keep a consistent distance from the centre of the area, as otherwise the
wheel will rotate at an uneven rate.

To press the horn, simply place your pen down near the centre of the area, as
indicated by the emblem in the middle. Release the pen to stop hornking.

You **should** try to get comfortable with the motions so that you can steer
quickly and properly even if you are panicked, or otherwise under pressure.

## Features

**not all are actually implemented yet, all these should be done before I merge
this to the main branch**


- Virtual steering wheel turned with a pointer.
- It has a horn in the centre.
- It can use a physical simulation for some fancier effects.
- Many properties are configurable.
- Supports a few mechanisms for getting input such as Evdev and UDP socket.
- Same for emulating the virtual controller (e.g. using uinput on Linux).
- The wheel can react to force-feedback data from games. This allows it to, for
instance, automatically return to the centre using the game's physics. This only
works for some output device implementations such as the uinput backend, as
others do not support handling force-feedback.
- todo ???

## Building

under construction

## blah

under construction

## Acknowledgements

- The [Slint toolkit](https://slint.dev/) powers the graphical user interface.
- The [Roboto Mono](https://github.com/googlefonts/robotomono) font.

## Licence

The code is licenced under [GPL-3.0-or-later](LICENSES/GPL-3.0-or-later.txt).

Most assets are [CC-BY-SA-4.0](LICENSES/CC-BY-SA-4.0.txt), with some exceptions.

Please see `LICENCES/`, [REUSE.toml](REUSE.toml), and
[assets/REUSE.toml](assets/REUSE.toml) for more information.
