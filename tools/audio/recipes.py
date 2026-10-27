"""What each game sound is made of.

A recipe is a list of layers cut from the Sonniss GDC 2024 bundle (paths are
relative to the bundle folder). `ev` picks the n-th separate hit that
dsp.find_events detects in the take, `t` is a start time in seconds, `segs`
splices several pieces of one take. Loudness targets are integrated LUFS of
the clip; quiet sounds (steps, UI) sit far below shots on purpose.
"""

SRC = {
    # guns
    "sw60": "Pole Position - Smith & Wesson .38 Chiefs Special Model 60/Smith & Wesson 60 - FIRING - Take 3 - 3m Behind Left - CO100K.wav",
    "glock_room": "Pole Position - The Indoor Gun Acoustics 2 Library/Glock 17 9mm - FIRING - Small Living Room - Single Shots - 5m In Front - MS - RSM191.wav",
    "python": "Pole Position - Colt Python .357 Magnum/Colt Python 357 Magnum - FIRING - Take 1 - 4m Left - CSS5.wav",
    "saiga_dsg": "Dramatic Cat - Saiga-12/GUNShotg_Saiga 12 Cal.12 DESIGNED Single Shot Core Long_DRCA_SG12_Stereo.wav",
    "saiga_mech": "Dramatic Cat - Saiga-12/GUNMech_Saiga 12 Cal.12 SOURCE Chamber Eject Bullet_DRCA_SG12_MKH416.wav",
    "spas_semi": "Pole Position - Franchi SPAS-12 Combat Shotgun/SPAS-12 - FIRING - Semi Automatic - Take 1 - 3m Behind Left - CO100K.wav",
    "spas_20m": "Pole Position - Franchi SPAS-12 Combat Shotgun/SPAS-12 - FIRING - Pump Action - Take 1 - 20m In Front - AB - MKH8020.wav",
    "spas_pump": "Pole Position - Franchi SPAS-12 Combat Shotgun/SPAS-12 - VAR SFX - Pump Action Fast - Close - MS - MKH418s.wav",
    "ak5_hall": "Pole Position - The Indoor Gun Acoustics 2 Library/AK5 7,62mm - FIRING - Large Hall - Long Bursts - 3m Behind Left - MS Decoded - MKH418S.wav",
    "svd_dsg": "Dramatic Cat - SVD Dragunov/GUNRif_SVD Dragunov 7.62×54R DESIGNED Single Shot Core Long_DRCA_DRAG_Stereo_05.wav",
    "svd_30m": "Dramatic Cat - SVD Dragunov/GUNRif_SVD Dragunov 7.62×54R SOURCE Single Shots 30m Front MikroUsi_DRCA_DRAG_AB.wav",
    "svd_mag": "Dramatic Cat - SVD Dragunov/GUNMech_SVD Dragunov 7.62×54R SOURCE Magazine Insert Slow_DRCA_DRAG_CO-100K.wav",
    "lynx_4m": "Pole Position - Lynx 50 Cal Sniper Rifle/Lynx 50 - FIRING - Take 2 - 4m Left - CSS5.wav",
    "lynx_shells": "Pole Position - Lynx 50 Cal Sniper Rifle/Lynx 50 - VAR SFX - Shell Ejections - Take 2 - CO100k.wav",
    "deagle_mag": "Pole Position - Desert Eagle .44/Desert Eagle - VAR SFX - Magazine In and Out - Take 3 - MKH8060.wav",
    "gloves": "Pole Position - Hammerli 280 22LR/Hämmerli 280 - VAR SFX - Grabs With Gloves - XY - RSM191.wav",
    # impacts, melee, explosions
    "shield_block": "Justsoundeffects - Melee Weapons/WEAPArmr_Metal Shield Block Hits_JSE_MW.wav",
    "esword": "Rogue Waves - Anime Studio/WEAPSwrd_Mecha Energy Sword Cut_RogueWaves_AnimeStudio.wav",
    "screech": "Mechanical Wave - Torturing Metal/METLFric_Screeching Metal Rub-11_MWSFX_TM.wav",
    "axe_flesh": "Justsoundeffects - Melee Weapons/WEAPAxe_Long Two-Handed Axe Flesh Hit_JSE_MW.wav",
    "scrapes": "Justsoundeffects - Melee Weapons/WEAPSwrd_Weapon 03 Slow Scrapes_JSE_MW.wav",
    "rip13": "InMotionAudio - Torn T-Shirt/CLOTHRip_CottonRips13_InMotionAudio_TornTShirt.wav",
    "rip44": "InMotionAudio - Torn T-Shirt/CLOTHRip_CottonRips44_InMotionAudio_TornTShirt.wav",
    "expl": "DavidDumais - Explosion SFX Pack/EXPLReal_Medium Realistic Explosion 15_DDUMAIS_NONE.wav",
    "mine": "Pole Position - The Warfare 2 Library/Tank Mine - EXPLOSION - Mounted on Metal Beam - DISTANT - AMBEO BLD.wav",
    "iron": "Pole Position - The Metal Hit Sweeteners Library/Iron - Thick - HIT - Hammer.wav",
    "barrel": "Pole Position - The Metal Hit Sweeteners Library/Oil Barrel - HIT - Hammer - Take 2.wav",
    "spade": "Pole Position - The Metal Hit Sweeteners Library/Spade - HIT - Drumstick - Ring - Mute.wav",
    "toolbox": "Pole Position - The Metal Hit Sweeteners Library/Toolbox - DROP.wav",
    "metal_imp": "Mechanical Wave - Torturing Metal/METLImpt_Metal Impact-03_MWSFX_TM.wav",
    "metal_debris": "Mechanical Wave - Torturing Metal/METLCrsh_Drop Fall Metal Rattle Scrap Debris_06_MWSFX_TM.wav",
    "pipe_thump": "Pole Position - The Plastic Textures Library/Flexible Sewer Pipe Big - Broken - DROP - Thump - Mono.wav",
    "ricochet": "Justsoundeffects - Steampunk Gadgets/MECHMisc_Ricochet Hits 01_JSE_SG.wav",
    "stone15": "BluezoneCorp - Stone Impact/Bluezone_BC0297_stone_impact_015.wav",
    "stone41": "BluezoneCorp - Stone Impact/Bluezone_BC0297_stone_impact_041.wav",
    "steelbar": "BluezoneCorp - Stone Impact/Bluezone_BC0297_stone_impact_steel_bar_01_010.wav",
    "rock_drop": "Sonic Bat - Videogame Foley Essentials Vol. II/SBvfe2_Medium Rock Dropping 011.wav",
    "glass_small": "Sonic Bat - Videogame Foley Essentials Vol. II/SBvfe2_Glass 114.wav",
    "glass_break": "Mechanical Wave - Glass/GLASBrk_Glass Break Hit_04_MWSFX_GL.wav",
    "box_imp": "Mechanical Wave - Cardboard and Paper/OBJPack_Big Cardboard Box Impact_ 02_MWSFX_CDAP.wav",
    # robot, foley, water, electricity
    "mech_steps": "DavidDumais - Robotic Creatures Sound FX/CREAMisc_Heavy Mechanical Footsteps 03_DDUMAIS_MCSFX.wav",
    "power_on": "DavidDumais - Robotic Creatures Sound FX/CREAMisc_Creature Power On 14_DDUMAIS_MCSFX.wav",
    "droid_tf": "Unreal SFX - Battle Droid/Droid-Transformation-07.wav",
    "neoprene": "TheWorkRoom Audio Post - Neoprene - Dry - Wetsuit/NEO008.wav",
    "face_shield": "InMotionAudio - Mask Foley/FOLYMisc_PlasticFaceShield_Movement04_InMotionAudio_MaskFoley.wav",
    "water_soft": "Justsoundeffects - Water in Motion/WATRMvmt_Soft Movement 01_JSE_WIM.wav",
    "arc": "BluezoneCorp - High Voltage/Bluezone_BC0299_electricity_surge_discharge_electrical_arc_crackling_002_01.wav",
    "sizzle": "BluezoneCorp - High Voltage/Bluezone_BC0299_electricity_texture_sizzling_crackling_006.wav",
    "lighter_click": "BluezoneCorp - High Voltage/Bluezone_BC0299_electricity_electronic_lighter_button_click_on_off_002_02.wav",
    "zap": "Rescopic Sound - Parallax/SCIMisc_Zap Short 14_RSCPC_PX.wav",
    "ping": "Rescopic Sound - Parallax/SCIMisc_Ping 18_RSCPC_PX.wav",
    "whoosh_air": "Rescopic Sound - Distinct Whooshes/WHSH_Airy-Whoosh Wind Gust 11_RSCPC_DW.wav",
    "whoosh_fast": "Rescopic Sound - Distinct Whooshes/WHSH_Pure SciFi-Whoosh Fast 03_RSCPC_DW.wav",
    "canteen": "Eiravaein Works - Flask/OBJCont_Flask,aluminum,canteen,medium,feltcover,leatherbanding,aluminumrealcork,liquid,grab,squeeze,slosh_EWKR.wav",
    # interface
    "gear_click": "BluezoneCorp - Tiny Gears - Small Mechanism/Bluezone_BC0301_tiny_gears_small_mechanism_click_003.wav",
    "gear_click2": "BluezoneCorp - Tiny Gears - Small Mechanism/Bluezone_BC0301_tiny_gears_small_mechanism_click_complex_011.wav",
    "gear_seq": "BluezoneCorp - Tiny Gears - Small Mechanism/Bluezone_BC0301_tiny_gears_small_mechanism_sequence_045.wav",
    "lever14": "BluezoneCorp - Industrial Lever Switch/Bluezone_BC0302_industrial_lever_switch_014.wav",
    "lever_small": "BluezoneCorp - Industrial Lever Switch/Bluezone_BC0302_industrial_lever_switch_small_003.wav",
    "comp_button": "BluezoneCorp - Retrofuturistic Computer/Bluezone_BC0304_retrofuturistic_computer_button_013.wav",
    "glitch": "BluezoneCorp - Retrofuturistic Computer/Bluezone_BC0304_retrofuturistic_computer_glitch_malfunction_004.wav",
    "data_glitch": "BluezoneCorp - Futuristic User Interface/Bluezone_BC0303_futuristic_user_interface_data_glitch_003.wav",
    "beep": "BluezoneCorp - Futuristic User Interface/Bluezone_BC0303_futuristic_user_interface_high_tech_beep_038.wav",
    "transition": "BluezoneCorp - Futuristic User Interface/Bluezone_BC0303_futuristic_user_interface_transition_006.wav",
    "boom": "BluezoneCorp - Modern Cinematic Impact/Bluezone_BC0294_modern_cinematic_impact_boom_003.wav",
    "trs": "Bolt - ARP 2600- Droids, Blips, Drones & more/MECHMisc_TRS Cable Plug In Solid Metal Thump_BOLT_ARP2600.wav",
    "feedback": "CB Sounddesign - Activation 2/UIMisc_Feedback 36 up_CB Sounddesign_ACTIVATION2.wav",
    "slide_metal": "Doex Studio - Qantum UI/UI_Window_MEDIUM_SlideMetal_02_CLOSED.wav",
    "window_open": "Rescopic Sound - User Interaction/UIMvmt_Window Open Thin 05_RSCPC_USIN.wav",
    "haunted": "Jake Fielding - Haunted Metal Vol.1 - Cinematic Hits & Impacts/DSGNBoom_Cinematic Metallic Hit, Boom, Trailer, Sub_JF_Haunted Metal Vol 1_02.wav",
    "interf_cable": "Pole Position - Electric Guitar Pickup Interference/Interference - Cable - Buzz Hum - Swells to Sputtering Noise.wav",
    "interf_pickup": "Pole Position - Electric Guitar Pickup Interference/Interference - Guitar Pickup - Buzz Hum - Thin - Swells - Hits Thumps.wav",
    # ambience
    "room_tone": "Jake Fielding - Chamber of Shadows - Morgue Room Tones/AMBRoom_Morgue Freezer Tone, Interior_JF_Morgue Tones_02.wav",
    "groan_creaky": "Jake Fielding - Haunted Metal Vol.3 - Cinematic Tones & Textures/DSGNRise_Tense Metallic Creaky Hits, Groan_JF_Haunted Metal Vol 3_14.wav",
    "groan_low": "Jake Fielding - Haunted Metal Vol.2 - Cinematic Creaks & Risers/DSGNRise_Cinematic Metallic Riser, Trailer, Designed Groan_JF_Haunted Metal Vol 2_08.wav",
    "hinge": "Jake Fielding - Squeaky Gates/DOORGate_Wooden Metal Hinge Creaks_Jake Fielding_Squeaky Gates.wav",
    "rails": "The Soundcatcher - Seismic Activities/Metal_Rails_Pack_Rumble_Rattle_Shake_Vibrate_Debris_Mkh8040_Seismic_Activities_03.Wav",
    "wind_concrete": "Sonik Sound Library - Spatial Countryside/WIND_Wind, Soft Wind Howling, Soft Leaves Rustle on Concrete_KS_Spatial Counstryside-Stereo_KSL_KS011.wav",
    "rain_thunder": "Sonik Sound Library - Spatial Rain & Thunders/THUN_Weather, Rain, Light Rain with Thuder Rumbles and Vehicles Passing on Wet Floor_KS_Spatial Rain & Thunders-Stereo_KSL_KS010.wav",
    "thunderstorm": "Bolt - Backyard Rain & Thunder - Suburban Rain Recordings/RAIN_Distant Thunder and Rain Long Thunderstorm C_BOLT_BackyardRain_UsiPro.wav",
    "amb_low": "Rescopic Sound - Parallax/AMBMisc_Ambience Low 03_RSCPC_PX.wav",
    "cooler": "Jake Fielding - Fridge Hums/MACHAppl_Water Cooler, Fridge, Electrical Appliance,_Jake Fielding_Fridge Hums_02.wav",
    "generator": "Jake Fielding - Circus Ambiences/MACHFan_Generator, Air Conditioning, Motor, Hum_JF_Circus_02.wav",
    "heli": "Pole Position - The Aircraft Ambient Library/Helicopter - Military - FLY - By - Overhead - ORTF - MKH8040.wav",
}


def L(src, ev=None, t=None, dur=None, segs=None, g=0.0, st=0.0, hp=None, lp=None,
      fi=0.002, fo=None, at=0.0, rev=False, align_peak=False, rel=-55):
    return dict(src=src, ev=ev, t=t, dur=dur, segs=segs, g=g, st=st, hp=hp, lp=lp,
                fi=fi, fo=fo, at=at, rev=rev, align_peak=align_peak, rel=rel)


def R(layers, loud=-18.0, room=0.0, rt60=1.4, hp=40, lp=None, stereo=False,
      loop=None, tail=-60, ceiling=-1.0, max_gr=3.0, punch=None):
    """`loud` is integrated LUFS; hits use `punch` instead (loudest 50 ms, dBFS),
    because a gunshot's tail drags its LUFS down. `max_gr` caps how hard the
    limiter may squash peaks, so a shot stops short of its target rather than
    losing its crack."""
    return dict(layers=layers, loud=loud, room=room, rt60=rt60, hp=hp, lp=lp,
                stereo=stereo, loop=loop, tail=tail, ceiling=ceiling, max_gr=max_gr, punch=punch)


def burst(start, rounds_end, tail_from, length=0.9):
    """A 3-round burst: the first rounds, then the hall tail after the long burst ends."""
    return [(start, rounds_end), (tail_from, tail_from + length)]


SFX = {}

# ---------- shots: "near" is our own shot from behind the shoulder, "far" is the enemy's across the hall ----------

for i, ev in enumerate([0, 2, 4], 1):
    SFX[f"shot-pistol-near-{i}"] = R([L("sw60", ev=ev, dur=1.0, st=[0, -0.3, 0.3][i - 1])], punch=-10, max_gr=4, room=0.16, rt60=1.5, hp=70)
for i, ev in enumerate([0, 4, 8], 1):
    SFX[f"shot-pistol-far-{i}"] = R([L("glock_room", ev=ev, dur=0.6)], punch=-16, room=0.32, rt60=1.7, hp=120, lp=7000)

for i, (ev, ev2) in enumerate([(1, 1), (4, 4), (6, 7)], 1):
    SFX[f"shot-shotgun-near-{i}"] = R([
        L("spas_semi", ev=ev, dur=1.0),
        L("saiga_dsg", ev=ev2, dur=0.6, g=-9, align_peak=True),
    ], punch=-8, max_gr=4, room=0.14, rt60=1.5, hp=60)
for i, ev in enumerate([0, 4, 6], 1):
    SFX[f"shot-shotgun-far-{i}"] = R([L("spas_20m", ev=ev, dur=1.2, rel=-50)], punch=-15, room=0.36, rt60=1.8, hp=110, lp=6500)

# AK in a large hall: the first 3 rounds of a long burst, spliced onto the hall tail.
_bursts = [burst(0.0, 0.335, 1.104), burst(0.336, 0.680, 1.104), burst(4.296, 4.588, 5.57), burst(4.588, 4.908, 5.57)]
for i, segs in enumerate(_bursts, 1):
    SFX[f"shot-rifle-near-{i}"] = R([L("ak5_hall", segs=segs)], punch=-10, max_gr=6, room=0.05, hp=70)
for i, segs in enumerate(_bursts[1:], 1):
    SFX[f"shot-rifle-far-{i}"] = R([L("ak5_hall", segs=segs, st=-0.2)], punch=-18, room=0.38, rt60=1.9, hp=150, lp=5000)

for i, (ev, st) in enumerate([(0, 0), (2, -0.4), (4, 0.3)], 1):
    SFX[f"shot-sniper-near-{i}"] = R([
        L("svd_dsg", ev=0, dur=2.6, st=st),
        L("lynx_4m", ev=ev, dur=1.2, g=-11, lp=3500, align_peak=True),
    ], punch=-8, max_gr=4, room=0.08, hp=55)
for i, ev in enumerate([0, 2, 5], 1):
    SFX[f"shot-sniper-far-{i}"] = R([L("svd_30m", ev=ev, dur=2.0, rel=-50)], punch=-14, room=0.3, rt60=2.0, hp=80)

for i, ev in enumerate([1, 5, 13], 1):
    SFX[f"casing-{i}"] = R([L("lynx_shells", ev=ev, dur=0.5)], punch=-26, hp=1500)

# ---------- knife ----------

for i, (st, ev) in enumerate([(4, 3), (6, 7)], 1):
    SFX[f"knife-swing-{i}"] = R([
        L("whoosh_fast", t=0.35, dur=0.42, st=st, hp=400, fi=0.03, fo=0.15),
        L("scrapes", ev=ev, dur=0.25, st=5, g=-12, hp=2000, at=0.12, fo=0.1),
    ], punch=-19, hp=200)
for i, ev in enumerate([0, 2, 4], 1):
    SFX[f"knife-hit-{i}"] = R([
        L("rip13", ev=0, dur=0.3, st=[0, 1, -1][i - 1]),
        L("axe_flesh", ev=ev, dur=0.35, g=-8, lp=2500),
        L("box_imp", ev=0, dur=0.2, g=-10),
    ], punch=-13, hp=80)
# a small blade glancing off the shield: a light tink, a short scrape, the shield's crackle
for i, (ev, st, t) in enumerate([(0, 4, 2.38), (2, 2, 2.30)], 1):
    SFX[f"knife-blocked-{i}"] = R([
        L("spade", ev=ev, dur=0.22, st=st, hp=1500, fo=0.15),
        L("screech", t=t, dur=0.14, g=-12, hp=2500, fi=0.01, fo=0.08, at=0.02),
        L("esword", t=0.07, dur=0.26, g=-9, hp=600, fo=0.18),
        L("sizzle", t=0.1, dur=0.2, g=-20, fo=0.15),
    ], punch=-17, hp=300)

# ---------- hits on a Persona ----------

for i, (b, p, ir) in enumerate([(1, 1, 1), (4, 5, 3), (7, 8, 6)], 1):
    SFX[f"hit-armor-{i}"] = R([
        L("barrel", ev=b, dur=0.35),
        L("pipe_thump", ev=p, dur=0.3, g=-4, lp=3000),
        L("iron", ev=ir, dur=0.12, g=-14),
    ], punch=-13, max_gr=5, hp=60)
for i, ev in enumerate([1, 3, 4], 1):
    SFX[f"hit-suit-{i}"] = R([
        L("box_imp", ev=0, dur=0.3, st=[0, -1, 1][i - 1]),
        L("axe_flesh", ev=ev, dur=0.3, g=-6, lp=3500),
        L("metal_imp", ev=0, dur=0.25, g=-16, lp=4000, st=[0, 2, -2][i - 1]),
    ], punch=-13, hp=70)
for i, ev in enumerate([0, 3], 1):
    SFX[f"hit-head-{i}"] = R([
        L("spade", ev=ev, dur=0.3),
        L("glass_small", ev=0, dur=0.4, g=-8, st=[0, 2][i - 1]),
    ], punch=-17, hp=200)
for i, ev in enumerate([1, 2], 1):
    SFX[f"hit-crit-{i}"] = R([
        L("arc", ev=ev, dur=0.36, hp=800),
        L("sizzle", t=0.05, dur=0.4, g=-10, fo=0.25),
    ], punch=-18)
for i, ev in enumerate([1, 3, 4], 1):
    SFX[f"shield-block-{i}"] = R([
        L("shield_block", ev=ev, dur=0.55),
        L("zap", t=0.0, dur=0.35, g=-8, hp=500, fo=0.25),
        L("sizzle", t=0.1, dur=0.25, g=-16, fo=0.2),
    ], punch=-12)
SFX["shield-break"] = R([
    L("glass_break", ev=0, dur=0.55),
    L("arc", ev=0, dur=0.34, g=-4),
    L("shield_block", ev=0, dur=0.6, g=-6),
], punch=-11)
for i, ev in enumerate([0, 2, 4], 1):
    SFX[f"miss-near-{i}"] = R([
        L("ricochet", ev=ev, dur=0.9, hp=1200, fo=0.4),
        L("stone15" if i != 2 else "stone41", ev=0, dur=0.3, g=-8),
    ], punch=-16)
for i, ev in enumerate([1, 4], 1):
    SFX[f"miss-far-{i}"] = R([
        L("ricochet", ev=ev, dur=0.8, hp=1500, fo=0.4),
        L("stone41", ev=0, dur=0.3, g=-6),
    ], punch=-22, room=0.3, lp=5000)

# ---------- grenade ----------

SFX["grenade-pin"] = R([L("gear_click2", ev=0, dur=0.25), L("lever_small", ev=0, dur=0.28, g=-6, at=0.12)], loud=-23)
SFX["grenade-throw"] = R([L("whoosh_air", t=1.1, dur=0.55, st=5, hp=300, fi=0.05, fo=0.25)], loud=-21)
SFX["grenade-bounce"] = R([
    L("rock_drop", ev=0, dur=0.25),
    L("steelbar", ev=0, dur=0.2, g=-10),
    L("rock_drop", ev=0, dur=0.25, st=2, g=-6, at=0.22),
], punch=-19)
SFX["grenade-blast"] = R([
    L("expl", ev=0, dur=2.36),
    L("mine", ev=0, dur=2.5, g=-6, lp=3000, align_peak=True),
    L("metal_debris", ev=0, dur=0.55, g=-10, at=0.35),
    L("toolbox", ev=1, dur=0.35, g=-16, at=0.6),
], punch=-9, max_gr=6, room=0.18, rt60=2.2, hp=35)

# ---------- moving, falling, the Persona itself ----------

# The bundle has no human footsteps: a boot is built from the first hit of a heavy
# mechanical step (pitched up, no sub), a little grit, a wet splash and suit rustle.
for i, (ev, w, st) in enumerate([(0, 2.0, 6), (1, 7.5, 5), (2, 13.0, 6.5), (3, 21.0, 5.5)], 1):
    SFX[f"step-{i}"] = R([
        L("mech_steps", ev=ev, dur=0.12, st=st, hp=140, fo=0.08),
        L("rock_drop", ev=0, dur=0.08, g=-12, hp=1500, st=i - 2),
        L("water_soft", t=w, dur=0.18, g=-14, hp=600, lp=7000, fi=0.01, fo=0.12),
        L("neoprene", t=[2.0, 5.3, 7.9, 14.4][i - 1], dur=0.12, g=-20, fo=0.08),
    ], punch=-25, hp=90)
SFX["dash-whoosh"] = R([L("whoosh_air", t=0.9, dur=0.65, st=2, hp=250, fi=0.06, fo=0.3)], loud=-24)
for i, t in enumerate([7.9, 23.47], 1):
    SFX[f"gear-rustle-{i}"] = R([L("neoprene", t=t, dur=0.3, fo=0.15)], loud=-27)
SFX["body-fall"] = R([
    L("pipe_thump", ev=5, dur=0.45),
    L("toolbox", ev=5, dur=0.4, g=-4, at=0.03),
    L("metal_debris", ev=0, dur=0.55, g=-6, at=0.12),
    L("box_imp", ev=0, dur=0.3, g=-6),
], punch=-12, room=0.12)
SFX["persona-powerdown"] = R([
    L("power_on", ev=0, dur=1.6, rev=True, st=-3, lp=6000, fi=0.05),
    L("droid_tf", ev=0, dur=2.4, rev=True, g=-10, st=-2),
], loud=-21)
SFX["persona-powerup"] = R([L("power_on", ev=0, dur=1.6), L("droid_tf", ev=0, dur=2.4, g=-10)], loud=-20)
SFX["sync-lost"] = R([
    L("glitch", t=0.5, dur=1.2, fo=0.5),
    L("data_glitch", t=0.14, dur=1.0, g=-6, fo=0.4),
], loud=-21, lp=7000)

# ---------- interface ----------

SFX["ui-tap"] = R([L("gear_click2", ev=0, dur=0.2)], loud=-26, hp=300)
SFX["ui-guard"] = R([L("lighter_click", ev=0, dur=0.07), L("zap", t=0, dur=0.15, g=-12, hp=1500, fo=0.1)], loud=-26)
SFX["ui-step"] = R([L("gear_click", t=0, dur=0.05, st=-2)], loud=-28, hp=300)
SFX["ui-confirm"] = R([L("lever14", t=0.2, dur=0.75, hp=150), L("comp_button", ev=0, dur=0.24, g=-8)], loud=-22)
SFX["ui-tick"] = R([L("gear_click", t=0, dur=0.05, st=-4)], loud=-29, hp=300)
SFX["ui-tick-urgent"] = R([L("beep", t=0, dur=0.08, st=-3)], loud=-27)
SFX["ui-open"] = R([L("window_open", t=0, dur=0.75)], loud=-27)
SFX["ui-close"] = R([L("slide_metal", t=0.45, dur=0.8)], loud=-27)
SFX["ui-turn"] = R([L("ping", t=0, dur=0.9, lp=6000, fo=0.5)], loud=-28)
SFX["ui-error"] = R([L("comp_button", ev=0, dur=0.2, st=-5), L("comp_button", ev=0, dur=0.2, st=-7, at=0.13)], loud=-25)
SFX["ui-match-found"] = R([L("transition", t=0.03, dur=1.03), L("boom", t=0, dur=2.0, g=-12, lp=200)], loud=-22)
SFX["ui-victory"] = R([L("feedback", t=0, dur=0.6), L("ping", t=0, dur=1.4, g=-6, fo=0.8)], loud=-22)
SFX["ui-defeat"] = R([L("haunted", t=0.55, dur=4.0, lp=3000, fo=2.0)], loud=-21)
SFX["loot-water"] = R([L("canteen", t=0.05, dur=1.05)], loud=-24)
SFX["loot-scrap"] = R([L("toolbox", ev=7, dur=0.35), L("gear_seq", t=0, dur=0.6, g=-10)], loud=-23)

# ---------- gear screen ----------

SFX["equip-pistol"] = R([L("deagle_mag", t=16.88, dur=0.7)], loud=-22, hp=120)
SFX["equip-shotgun"] = R([L("spas_pump", t=4.10, dur=0.75)], loud=-21, hp=120)
SFX["equip-rifle"] = R([L("saiga_mech", ev=2, dur=0.6), L("gloves", ev=16, dur=0.12, g=-6)], loud=-21, hp=120)
SFX["equip-sniper"] = R([L("svd_mag", ev=0, dur=0.72)], loud=-22, hp=120)
SFX["equip-knife"] = R([L("scrapes", ev=4, dur=0.9)], loud=-23)
SFX["equip-grenade"] = R([L("gear_click2", ev=0, dur=0.25), L("lever_small", ev=0, dur=0.28, g=-4, at=0.08)], loud=-24)
SFX["equip-armor"] = R([L("rip44", ev=0, dur=0.4), L("barrel", ev=2, dur=0.2, g=-14, at=0.25)], loud=-23)
SFX["equip-helmet"] = R([L("face_shield", t=0.5, dur=0.6, fo=0.3)], loud=-24)
SFX["equip-suit"] = R([L("neoprene", t=6.68, dur=0.87, fo=0.3)], loud=-25)
SFX["equip-boots"] = R([L("box_imp", ev=0, dur=0.3), L("rock_drop", ev=0, dur=0.2, g=-8, at=0.04)], loud=-24)
SFX["equip-chevron"] = R([L("rip13", ev=0, dur=0.3)], loud=-24)

# ---------- radio in the operator's room ----------

SFX["radio-on"] = R([L("lighter_click", ev=0, dur=0.07), L("interf_cable", t=60.0, dur=0.5, g=-6, fo=0.3, at=0.03)], loud=-27, hp=400, lp=3500)
SFX["radio-off"] = R([L("trs", ev=2, dur=0.14), L("interf_pickup", t=30.0, dur=0.25, g=-8, fo=0.2)], loud=-27, hp=400, lp=3500)
SFX["radio-tune"] = R([L("interf_pickup", t=20.0, dur=0.8, fi=0.05, fo=0.3)], loud=-29, hp=400, lp=3500)

# ---------- one-shot events ----------

SFX["thunder-1"] = R([L("thunderstorm", t=75.0, dur=7.0, fi=0.3, fo=2.5)], loud=-19, stereo=True, hp=30)
SFX["thunder-2"] = R([L("thunderstorm", t=3.4, dur=6.0, fi=0.3, fo=2.5)], loud=-20, stereo=True, hp=30)
SFX["helicopter-flyby"] = R([L("heli", t=8.0, dur=20.0, fi=3.0, fo=5.0)], loud=-25, stereo=True, lp=6000)
SFX["artillery-far"] = R([L("mine", ev=0, dur=3.0, lp=900)], loud=-25, room=0.5, rt60=3.0)

# ---------- ambience loops (stereo, crossfaded so they repeat without a seam) ----------

AMB = {
    # the abandoned salt factory hall: dry and empty, wind through broken windows high up,
    # the low hum of a big concrete room, the steel roof creaking now and then
    "amb-hall": R([
        L("wind_concrete", t=12.0, dur=61.0, hp=250, lp=3500, fi=2.0, fo=2.0),
        L("room_tone", t=20.0, dur=61.0, g=-42, hp=35, lp=250, fi=2.0, fo=2.0),
        L("groan_creaky", t=0.3, dur=7.0, g=-28, lp=2500, fi=0.5, fo=2.5, at=7.0),
        L("groan_low", t=2.1, dur=4.6, g=-39, fi=0.5, fo=2.0, at=33.0, st=1),
        L("rails", t=2.0, dur=12.0, g=-28, lp=1200, fi=2.0, fo=3.0, at=21.0),
        L("hinge", t=1.0, dur=4.0, g=-31, lp=3000, fi=0.3, fo=1.0, at=46.0),
    ], loud=-30, stereo=True, loop=57.0, room=0.3, rt60=2.8, hp=30),
    # outside among ruined panel blocks
    "amb-blocks": R([
        L("wind_concrete", t=30.0, dur=62.0),
        L("rails", t=5.0, dur=26.0, g=-20, lp=1200, fi=2.0, fo=2.0, at=20.0),
    ], loud=-29, stereo=True, loop=60.0, hp=30),
    # weather layers on top of the location bed
    "amb-storm": R([L("rain_thunder", t=20.0, dur=62.0)], loud=-27, stereo=True, loop=60.0, hp=30),
    "amb-dust": R([L("wind_concrete", t=40.0, dur=62.0, st=-2, hp=150)], loud=-27, stereo=True, loop=60.0),
    "amb-fog": R([L("amb_low", t=0.0, dur=34.0)], loud=-33, stereo=True, loop=31.0, hp=30),
    # the operator's room behind the front: water cooler, generator, far-off shelling
    "amb-base": R([
        L("cooler", t=10.0, dur=62.0),
        L("generator", t=0.0, dur=59.0, g=-10, fi=2.0, fo=2.0),
        L("mine", ev=0, dur=3.0, g=-16, lp=800, at=17.0),
        L("mine", ev=0, dur=3.0, g=-20, lp=600, at=43.0, st=-1),
    ], loud=-32, stereo=True, loop=60.0, hp=30),
}
