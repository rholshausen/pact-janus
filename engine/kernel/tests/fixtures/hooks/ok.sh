#!/bin/sh
# A hook that did its job and had nothing to say: exit 0, empty stdout (spec §8.3).
while read -r _line; do :; done
exit 0
