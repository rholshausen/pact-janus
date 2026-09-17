#!/bin/sh
# A hook that tries to change something its entry never declared: the engine must refuse it and
# fail the exchange with `hook-change-refused` rather than quietly rewriting the request.
while read -r _line; do :; done
printf '{"outcome":"ok","changes":{"parts.request.path":{"content":"/somewhere-else"}}}'
