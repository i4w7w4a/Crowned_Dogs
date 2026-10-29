import assert from "node:assert/strict";
import { test } from "node:test";
import { run_server_main } from "../../../packages/runtime/src/index.ts";

test("Rust server startup installs commands and removes shutdown listeners after stopping", async () => {
  const signals = new Map<string, () => void>();
  const calls: string[] = [];
  let registryFile = "";
  const processHost = {
    env: { BOT_TOKEN: " test-token ", OPERATORS_FILE: " data/test-operators.json " },
    once(signal: string, callback: () => void) { signals.set(signal, callback); },
    removeListener(signal: string, callback: () => void) {
      assert.equal(signals.get(signal), callback);
      signals.delete(signal);
    },
    exit() { assert.fail("mock startup must not exit"); },
  };
  class Registry {
    constructor(opts: { file: string }) { registryFile = opts.file; }
  }
  await run_server_main({
    process: processHost,
    OperatorRegistry: Registry,
    createBot(token: string, opts: { operators: Registry }) {
      assert.equal(token, "test-token");
      assert.ok(opts.operators instanceof Registry);
      return {
        duels: { close() { calls.push("close"); } },
        bot: {
          async init() { calls.push("init"); },
          api: { async setMyCommands(commands: { command: string }[]) {
            assert.deepEqual(commands.map(c => c.command), ["fight", "duel", "rules", "me"]);
            calls.push("commands");
          } },
          async start() {
            assert.equal(signals.size, 2);
            calls.push("start");
            signals.get("SIGINT")!();
          },
          async stop() { calls.push("stop"); },
        },
      };
    },
  });
  assert.equal(registryFile, "data/test-operators.json");
  assert.deepEqual(calls, ["init", "commands", "start", "close", "stop"]);
  assert.equal(signals.size, 0);
});
