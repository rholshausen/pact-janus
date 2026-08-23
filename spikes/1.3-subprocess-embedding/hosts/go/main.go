// Go host leg: same scenarios as hosts/node/main.mjs.
// `go run .` — all scenarios; `go run . --runner` — orphan helper.
package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"
)

const engine = "../../engine-bin/target/release/pact-engine-toy"

type client struct {
	cmd    *exec.Cmd
	stdin  io.WriteCloser
	stdout *bufio.Reader
}

func spawnEngine() *client {
	cmd := exec.Command(engine)
	cmd.Stderr = nil
	stdin, err := cmd.StdinPipe()
	if err != nil {
		panic(err)
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		panic(err)
	}
	if err := cmd.Start(); err != nil {
		panic(err)
	}
	return &client{cmd, stdin, bufio.NewReader(stdout)}
}

func (c *client) request(frame string) map[string]json.RawMessage {
	fmt.Fprintf(c.stdin, "Content-Length: %d\r\n\r\n%s", len(frame), frame)
	contentLength := -1
	for {
		line, err := c.stdout.ReadString('\n')
		if err != nil {
			panic(err)
		}
		line = strings.TrimRight(line, "\r\n")
		if line == "" {
			break
		}
		if v, found := strings.CutPrefix(line, "Content-Length:"); found {
			contentLength, err = strconv.Atoi(strings.TrimSpace(v))
			if err != nil {
				panic(err)
			}
		}
	}
	body := make([]byte, contentLength)
	if _, err := io.ReadFull(c.stdout, body); err != nil {
		panic(err)
	}
	var resp map[string]json.RawMessage
	if err := json.Unmarshal(body, &resp); err != nil {
		panic(err)
	}
	return resp
}

func ok(cond bool, msg string) {
	if !cond {
		panic("FAIL: " + msg)
	}
	fmt.Println("  ok: " + msg)
}

func pidAlive(pid int) bool {
	return syscall.Kill(pid, 0) == nil
}

func stats(name string, samples []time.Duration) {
	s := append([]time.Duration(nil), samples...)
	sort.Slice(s, func(i, j int) bool { return s[i] < s[j] })
	at := func(q float64) float64 { return float64(s[int(q*float64(len(s)-1))].Nanoseconds()) / 1e3 }
	fmt.Printf("  %s: median %.1fµs  p95 %.1fµs  min %.1fµs\n", name, at(0.5), at(0.95), at(0))
}

func benchOp(c *client, name, frame string, warmup, iters int) {
	for i := 0; i < warmup; i++ {
		c.request(frame)
	}
	samples := make([]time.Duration, iters)
	for i := 0; i < iters; i++ {
		t0 := time.Now()
		c.request(frame)
		samples[i] = time.Since(t0)
	}
	stats(name, samples)
}

func main() {
	doc, err := os.ReadFile("../../../1.2-wasm-embedding/payloads/order-100kb.json")
	if err != nil {
		panic(err)
	}
	frames := map[string]string{
		"handshake":     `{"op":"handshake","protocol-versions":[1]}`,
		"handshake-bad": `{"op":"handshake","protocol-versions":[99]}`,
		"echo-small":    `{"op":"echo","payload":{"ping":1}}`,
		"echo-100k":     `{"op":"echo","payload":` + string(doc) + `}`,
		"match-100k":    `{"op":"match-type","expected":` + string(doc) + `,"actual":` + string(doc) + `}`,
		"shutdown":      `{"op":"shutdown"}`,
	}

	if len(os.Args) > 1 && os.Args[1] == "--runner" {
		c := spawnEngine()
		c.request(frames["handshake"])
		fmt.Println(c.cmd.Process.Pid)
		time.Sleep(60 * time.Second)
		os.Exit(1)
	}

	fmt.Println("scenario: handshake")
	c := spawnEngine()
	bad := c.request(frames["handshake-bad"])
	ok(strings.Contains(string(bad["err"]), "protocol-version-unsupported"), "unsupported version rejected: "+string(bad["err"]))
	good := c.request(frames["handshake"])
	ok(strings.Contains(string(good["ok"]), `"protocol-version":1`), "version 1 negotiated")
	c.cmd.Process.Kill()
	c.cmd.Wait()

	fmt.Println("scenario: spawn-to-ready")
	ready := make([]time.Duration, 10)
	for i := range ready {
		t0 := time.Now()
		c := spawnEngine()
		c.request(frames["handshake"])
		ready[i] = time.Since(t0)
		c.stdin.Close()
		c.cmd.Wait()
	}
	stats("spawn-to-ready", ready)

	fmt.Println("scenario: pipe benchmark")
	c = spawnEngine()
	c.request(frames["handshake"])
	benchOp(c, "echo-small", frames["echo-small"], 500, 5000)
	benchOp(c, "echo-100k", frames["echo-100k"], 50, 500)
	benchOp(c, "match-100k", frames["match-100k"], 20, 200)
	c.stdin.Close()
	c.cmd.Wait()

	fmt.Println("scenario: clean shutdown")
	c = spawnEngine()
	c.request(frames["handshake"])
	ack := c.request(frames["shutdown"])
	ok(strings.Contains(string(ack["ok"]), `"shutting-down":true`), "shutdown acknowledged")
	err = c.cmd.Wait()
	ok(err == nil && c.cmd.ProcessState.ExitCode() == 0, fmt.Sprintf("exit code 0 (got %d)", c.cmd.ProcessState.ExitCode()))
	ok(!pidAlive(c.cmd.Process.Pid), "no process left")

	fmt.Println("scenario: EOF exit")
	c = spawnEngine()
	c.request(frames["handshake"])
	c.stdin.Close()
	done := make(chan int, 1)
	go func() { c.cmd.Wait(); done <- c.cmd.ProcessState.ExitCode() }()
	select {
	case code := <-done:
		ok(code == 0, fmt.Sprintf("engine exited on EOF with code 0 (got %d)", code))
	case <-time.After(2 * time.Second):
		panic("FAIL: engine did not exit on EOF")
	}

	fmt.Println("scenario: orphan on runner SIGKILL")
	self, _ := filepath.Abs(os.Args[0])
	runner := exec.Command(self, "--runner")
	runnerOut, _ := runner.StdoutPipe()
	if err := runner.Start(); err != nil {
		panic(err)
	}
	var enginePid int
	fmt.Fscanln(runnerOut, &enginePid)
	ok(pidAlive(enginePid), fmt.Sprintf("engine (pid %d) alive under runner (pid %d)", enginePid, runner.Process.Pid))
	syscall.Kill(runner.Process.Pid, syscall.SIGKILL)
	waited := 0
	for pidAlive(enginePid) && waited < 5000 {
		time.Sleep(50 * time.Millisecond)
		waited += 50
	}
	ok(!pidAlive(enginePid), fmt.Sprintf("engine exited within %dms of runner SIGKILL", waited))
	runner.Wait()

	fmt.Println("all scenarios passed")
}
