#!/bin/sh
# Fit armour batch 2 (Tripo, 2026-11-01) onto both Personas. Git Bash on Blade:
#   sh run_batch2.sh male|female   (one lane per sex; each step adds one garment file in $O)
# BL runs Blender 5.2 with scipy (pip install --target ~/.crowned-dogs/blender-site scipy, with Blender's own python).
set -e
A=$(cd "$(dirname "$0")" && pwd -W)
CD=C:/Users/qtas5/Documents/CrownedDogs
P=$CD/project-files/crowned-dogs/3d
T=$CD/3d/04-armor/tripo/batch2
O=${O:-$CD/3d/04-armor/fit2}
BL() { PYTHONPATH="C:/Users/qtas5/.crowned-dogs/blender-site" "/c/Program Files/Blender Foundation/Blender 5.2/blender.exe" \
  -b --factory-startup --python-use-system-env --python-exit-code 1 --python "$@"; }
FIT() { BL "$A/fit_garment.py" -- "$@"; }
mkdir -p "$O/check"
if [ "$1" = male ]; then
  PB=$P/persona-male-art/male.blend
  BL "$A/cut_suit.py" -- "$P/armor/fitted/male-armor.blend" "$O/m-suitcut.blend" 0.27
  FIT "$PB" "$T/ryvok_male_P2_22790dec.glb" "$O/m-ryvok.blend" Ryvok cut_head=1 hide_below=auto cut_below=0.27 slot=suit
  FIT "$PB" "$T/cap_P2_7438b931.glb" "$O/m-cap.blend" Cap mode=rigid width=0.192 refine=0 top_gap=0.008 slot=head
  FIT "$PB" "$T/sfera_P2_b0b927f2.glb" "$O/m-sfera.blend" Sfera mode=rigid width=0.262 inside=-0.008 scale_prior=5 slot=head
  FIT "$PB" "$T/plita4_P2_6954e4de.glb" "$O/m-plita4.blend" Plita4 base="$O/m-suitcut.blend" \
      align=torso height=0.56 top_z=1.52 fit_bones= inside=-0.008 scale_prior=30 pen_w=3 aniso=1 refine=spine_01,spine_02,spine_03 slot=body push=none
  FIT "$PB" "$P/armor/tripo/vest.glb" "$O/m-vest.blend" Vest base="$O/m-suitcut.blend" \
      align=torso height=0.41 top_z=1.533 fit_bones= inside=-0.008 scale_prior=30 pen_w=3 aniso=1 refine=spine_01,spine_02,spine_03 slot=body push=none
  FIT "$PB" "$T/boots_P2_f1ec820e.glb" "$O/m-boots.blend" Boots pair=1 fit_bones= slab=0.05,0.25 \
      ref_zmax=0.30 refine=foot_l,foot_r,calf_l,calf_r inside=0.008 aniso=1 scale_prior=20 push=soft hide_below=0.30 slot=boots
else
  PB=$P/persona-female/fem.blend
  BL "$A/cut_suit.py" -- "$P/armor/fitted/female-armor.blend" "$O/f-suitcut.blend" 0.26
  FIT "$PB" "$T/ryvok_female_P2_7ffc5848.glb" "$O/f-ryvok.blend" Ryvok cut_head=1 hide_below=auto head_max=0.45 neck_cut=1.472 \
      cut_below=0.26 slot=suit
  FIT "$PB" "$T/cap_P2_7438b931.glb" "$O/f-cap.blend" Cap mode=rigid width=0.205 refine=0 top_gap=0.004 slot=head \
      hair=FemHairCards,FemHairCap
  FIT "$PB" "$T/sfera_P2_b0b927f2.glb" "$O/f-sfera.blend" Sfera mode=rigid width=0.264 inside=-0.010 scale_prior=5 slot=head \
      hair=FemHairCards,FemHairCap
  FIT "$PB" "$T/plita4_P2_6954e4de.glb" "$O/f-plita4.blend" Plita4 base="$O/f-suitcut.blend" \
      align=torso height=0.50 top_z=1.42 lock_z=1 fit_bones= inside=-0.008 scale_prior=30 pen_w=3 aniso=1 refine=spine_01,spine_02,spine_03 slot=body push=none
  FIT "$PB" "$P/armor/tripo/vest.glb" "$O/f-vest.blend" Vest base="$O/f-suitcut.blend" \
      align=torso height=0.39 top_z=1.47 fit_bones= inside=-0.008 scale_prior=30 pen_w=3 aniso=1 refine=spine_01,spine_02,spine_03 slot=body push=none
  FIT "$PB" "$T/boots_P2_f1ec820e.glb" "$O/f-boots.blend" Boots pair=1 fit_bones= slab=0.05,0.25 \
      ref_zmax=0.29 refine=foot_l,foot_r,calf_l,calf_r inside=0.008 aniso=1 scale_prior=20 push=soft hide_below=0.29 slot=boots
fi
echo LANE DONE $1
# Then, per sex (s=male|female, x=m|f, CUT=0.27|0.26), into the game model (node steps from a folder with
# @gltf-transform/core, @gltf-transform/extensions and meshoptimizer installed):
#   BL dress_gear.py -- $P/armor-game/{male-armor-anim,fem-armor-anim}.blend $O $x $CUT gear-$s.glb
#   gltf-transform resize --width 1024 --height 1024, then webp --quality 88   -> gear-$s-1kw.glb
#   node merge_gear.mjs persona-$s.glb gear-$s-1kw.glb merged.glb [ONLY=Plita4,Vest]
#   node tuck_hems.mjs merged.glb tucked.glb      (trouser legs into the boots)
#   node tuck_hair.mjs tucked.glb out.glb         (hair under the Cap and Sfera; female only)
#   research/carry-fit/boots/elbowsmooth.mjs out.glb out.glb Ryvok l|r   (Ryvok elbows: female l, male l and r)
