import assert from "node:assert/strict";
import { test } from "node:test";
import { Client } from "pg";
import type { PoolConfig } from "pg";
import { nativePoolConfigs } from "./online-native-config.ts";

const schema = "online_test_0123456789abcdef0123456789abcdef";
// Inspect pg's effective connection configuration without opening a socket.
const effective = (config: PoolConfig) => (new Client(config) as unknown as {
  connectionParameters: { host: string; database: string; options: string };
}).connectionParameters;

test("native test configuration refuses connection-string host overrides", () => {
  assert.throws(() => nativePoolConfigs("postgres://localhost/crowned_dogs_test?host=example.invalid", schema), /query parameters/i);
});

test("native test configuration refuses connection-string schema overrides", () => {
  assert.throws(() => nativePoolConfigs("postgres://localhost/crowned_dogs_test?options=-c%20search_path%3Dpublic", schema), /query parameters/i);
  assert.throws(() => nativePoolConfigs("postgres://localhost/crowned_dogs_test?host=example.invalid&options=-c%20search_path%3Dpublic", schema), /query parameters/i);
});

test("native test pools retain a validated local target and generated application schema", () => {
  for (const [authority, host] of [["localhost", "localhost"], ["127.0.0.1", "127.0.0.1"], ["[::1]", "::1"]]) {
    const config = nativePoolConfigs(`postgres://${authority}/crowned_dogs_test_isolated`, schema);
    for (const pool of [config.admin, config.application]) {
      assert.equal(effective(pool).host, host);
      assert.equal(effective(pool).database, "crowned_dogs_test_isolated");
    }
    assert.equal(effective(config.application).options, `-c search_path=${schema}`);
  }
  assert.throws(() => nativePoolConfigs("postgres://example.invalid/crowned_dogs_test", schema), /loopback/i);
  assert.throws(() => nativePoolConfigs("postgres://localhost/real_game", schema), /dedicated/i);
  assert.throws(() => nativePoolConfigs("socket://localhost/crowned_dogs_test", schema), /protocol/i);
  assert.throws(() => nativePoolConfigs("postgres://localhost/crowned_dogs_test", "public"));
});
