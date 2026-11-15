# frozen_string_literal: true

# The engine as a WASM component (engine/wasm, `janus-engine.wasm` in a release), hosted by Ruby's
# wasmtime gem as a Pact Broker would host it (staged plan S4.3a, ADR 0092).
#
#   ruby pipe.rb <janus-engine.wasm>
#
# Each line on stdin is one engine-protocol request frame; each line on stdout is the frame the
# engine answered it with. The first frame must be `engine/hello` (engine-protocol spec §5.1).
# `engine/component-host/tests/engine_wasm.rs` drives it, when JANUS_RUBY names a Ruby that has the
# Gemfile's gems, and holds its answers to the native engine's.
#
# The guest is granted nothing: no environment, no arguments, no files, no sockets, no inherited
# stdio. `subsumption/check` and `subsumption/decide` are pure functions of the documents in the
# frame, and the date a decision is judged against is the frame's `as-of`, never the guest's clock.

require "json"
require "wasmtime"

# The interface the world a release ships exports (engine/wasm/wit/engine.wit).
PIPE = ["pact:janus-engine/pipe@0.1.0", "call"].freeze

engine = Wasmtime::Engine.new
component = Wasmtime::Component::Component.from_file(engine, ARGV.fetch(0))
linker = Wasmtime::Component::Linker.new(engine)
Wasmtime::WASI::P2.add_to_linker_sync(linker)
store = Wasmtime::Store.new(engine, wasi_config: Wasmtime::WasiConfig.new)
call = linker.instantiate(store, component).get_func(PIPE)

$stdout.binmode
$stdin.each_line do |line|
  # A `list<u8>` crosses the gem as an Array of Integers, both ways.
  response = call.call(line.chomp.bytes)
  $stdout.write(response.pack("C*"), "\n")
  $stdout.flush
end
