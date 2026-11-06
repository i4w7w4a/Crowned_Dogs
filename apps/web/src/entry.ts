import "./style.css";
import "./base/base.css";
import "./base/live.css";
import { mount_application, unmount_application } from "../../../packages/runtime/src/index.ts";
import { BaseScreen } from "./base/base-screen.ts";
import { loadLocalLoadout } from "./base/loadout-state.ts";
import { MawLoader } from "./loading/maw-loader.ts";
import { gameAudio } from "./audio/arena-audio.ts";
import { mountSoundSettings, showSoundSettings } from "./base/sound-settings.ts";
import { connectOnline } from "./online/session.ts";

const online = await connectOnline();
mount_application({ BaseScreen, MawLoader, gameAudio,
  loadLocalLoadout, mountSoundSettings, showSoundSettings, resumeOnline: !!online.activity?.battle, loadBattle: () => import("./main.ts") });
if (import.meta.hot) import.meta.hot.dispose(unmount_application);
