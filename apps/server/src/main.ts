/** Node entry point: startup, commands and graceful shutdown are orchestrated by Rust. */
import { run_server_main } from "../../../packages/runtime/src/index.ts";
import { createBot } from "./bot.ts";
import { OperatorRegistry } from "./operators.ts";
await run_server_main({ process, createBot, OperatorRegistry });
