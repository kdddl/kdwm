#! /usr/bin/env bash

pid=$$
pgrep -fi startup.sh | grep -v "^$pid$" | xargs -I{} kill {};

setxkbmap -option caps:escape; 

~/.screenlayout/default.sh;

pkill -fi picom; picom;
pkill -fi dunst; dunst;
feh --bg-max ~/.wallpaper/synthblocks.png;
