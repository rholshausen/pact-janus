#!/bin/sh
# A hook that could not do its job: non-zero exit, with the reason on stderr (spec §8.3).
while read -r _line; do :; done
printf 'the token endpoint is unreachable\n' >&2
exit 3
