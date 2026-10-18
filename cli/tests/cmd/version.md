# Versions

The release workflow's smoke test reads `janus version`, so its shape is fixed: the binary's name,
the release version, and the engine protocol it speaks. `--version` says the same, and so does
`janus-engine`.

```console
$ janus version
janus [..] (engine protocol v1)

$ janus --version
janus [..] (engine protocol v1)

$ janus-engine --version
janus-engine [..] (engine protocol v1)

```
