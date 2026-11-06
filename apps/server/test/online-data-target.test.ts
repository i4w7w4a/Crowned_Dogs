import assert from "node:assert/strict";
import { test } from "node:test";
import { dataTarget } from "../src/online/data-target.ts";

test("operator tools pin an explicit loopback database and refuse URL overrides", () => {
  assert.deepEqual(dataTarget("postgresql://operator:synthetic@127.0.0.1:55432/online", "online"),
    { host: "127.0.0.1", port: 55432, database: "online", user: "operator", password: "synthetic" });
  for (const url of ["postgresql://u:p@127.0.0.1/other", "postgresql://u:p@remote/online",
    "postgresql://u:p@localhost/online?host=remote", "postgresql://u:p@localhost/online?options=-csearch_path=other",
    "postgresql://u:p@localhost/online#fragment", "postgresql://u:p@localhost/on%6cine", "postgresql://localhost/online"]) {
    assert.throws(() => dataTarget(url, "online"));
  }
});
