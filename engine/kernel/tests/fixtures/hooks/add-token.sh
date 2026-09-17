#!/bin/sh
# A `before-request` hook (lifecycle-hooks spec §8.3): read the context from stdin, answer with a
# change on stdout. Shell built-ins only — an exec hook's environment is exactly what its entry
# declares (deny-by-default, ADR 0013), so nothing here may assume a PATH it did not ask for.
while read -r _line; do :; done
printf '{"outcome":"ok","changes":{"parts.request.headers":{"content":{"authorization":["Bearer %s"]}}}}' "$TOKEN"
