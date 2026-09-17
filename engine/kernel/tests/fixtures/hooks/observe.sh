#!/bin/sh
# A hook that succeeded and had nothing to say: exit 0 with empty stdout is `{"outcome":"ok"}`.
while read -r _line; do :; done
exit 0
