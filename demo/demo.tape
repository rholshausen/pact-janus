# The Pact Janus demo as a screencast (plan task 9.3). Render from the repo root with
#
#   vhs demo/demo.tape
#
# which writes demo/demo.gif. VHS needs ttyd and ffmpeg on the PATH. Everything on screen is
# demo/run.sh, paced; nothing is typed that the script does not do.

Output demo/demo.gif

Set Shell "bash"
Set FontSize 15
Set LineHeight 1.25
Set Width 1400
Set Height 980
Set Padding 24
Set TypingSpeed 35ms
Set Theme "Catppuccin Mocha"

# Build before the camera rolls, so the recording starts at the demo, not at cargo.
Hide
Type "cargo build -q -p pact_janus_cli -p pact_janus_sample_order_service && { [ -d demo/web-app/node_modules ] || (cd demo/web-app && npm ci --silent --no-audit --no-fund); } && clear"
Enter
Wait@600s
Show

Type "demo/run.sh --pace 5"
Sleep 500ms
Enter
Wait+Screen@300s /demo complete/
Sleep 4s
