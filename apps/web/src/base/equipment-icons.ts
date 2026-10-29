/** Original vector assets selected by the shared Rust equipment policy. */
import type { GearSlot } from "@crowned-dogs/core";
import { equipment_icon, equipment_item_icon, fitting_room_icon } from "../../../../packages/runtime/src/index.ts";
export function equipmentIcon(slot: GearSlot, itemId?: string | null): string { return equipment_icon(slot, itemId ?? undefined); }
export function equipmentItemIcon(itemId: string): string { return equipment_item_icon(itemId); }
export function fittingRoomIcon(): string { return fitting_room_icon(); }
