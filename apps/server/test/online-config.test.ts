import assert from "node:assert/strict";
import { test } from "node:test";
import { onlineConfig, onlineRuntimeConfig } from "../src/online/config.ts";

test("online config defaults to loopback and rejects incomplete or unsafe startup config", () => {
  const env = { ONLINE_DATABASE_URL: "synthetic-test-url", BOT_TOKEN: "synthetic-token", ONLINE_ORIGIN: "https://game.example" };
  assert.deepEqual(onlineConfig(env), { databaseUrl: env.ONLINE_DATABASE_URL, botToken: env.BOT_TOKEN,
    origin: env.ONLINE_ORIGIN, host: "127.0.0.1", port: 3100, basePath: "/api/online", production: false });
  assert.equal(onlineConfig({ ...env, API_BASE_PATH: "/r2/api/online", NODE_ENV: "production" }).basePath, "/r2/api/online");
  assert.equal(onlineConfig({ ...env, NODE_ENV: "production" }).production, true);
  for (const port of ["0", "65536", "3x", "3.5", ""]) assert.throws(() => onlineConfig({ ...env, ONLINE_PORT: port }), /port/i);
  for (const key of ["ONLINE_DATABASE_URL", "BOT_TOKEN", "ONLINE_ORIGIN"]) assert.throws(() => onlineConfig({ ...env, [key]: "" }), /required/i);
});
test("online runtime requires an explicit forum topic and bot identity",()=>{
  const env={ONLINE_DATABASE_URL:"synthetic-test-url",BOT_TOKEN:"synthetic-token",ONLINE_ORIGIN:"https://game.example",ONLINE_BOT_USERNAME:"GameTestBot",ONLINE_CHAT_ID:"-1001",ONLINE_THREAD_ID:"566",ONLINE_WEBAPP_URL:"https://game.example/r2/"};
  for(const key of ["ONLINE_BOT_USERNAME","ONLINE_CHAT_ID","ONLINE_THREAD_ID","ONLINE_WEBAPP_URL"])assert.throws(()=>onlineRuntimeConfig({...env,[key]:""}));
  for(const thread of ["0","-1","","5x"])assert.throws(()=>onlineRuntimeConfig({...env,ONLINE_THREAD_ID:thread}));
  assert.equal(onlineRuntimeConfig(env).threadId,566);
  assert.equal(onlineRuntimeConfig(env).chatId,"-1001");
  assert.throws(()=>onlineRuntimeConfig({...env,ONLINE_WEBAPP_URL:"https://other.example/r2/"}));
});
