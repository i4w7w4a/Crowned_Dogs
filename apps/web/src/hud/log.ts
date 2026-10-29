import { RustFightLog, callRust } from "../../../../packages/runtime/src/index.ts";
export const icon = (id: string, cls = ""): string => callRust("ui.icon", [id, cls]);
export { RustFightLog as FightLog };
