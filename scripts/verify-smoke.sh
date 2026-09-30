#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mode="${1:?Usage: verify-smoke.sh native|offscreen SCENARIO...}"
shift
case "$mode" in
    native) mode_args=(--smoke-test) ;;
    offscreen) mode_args=(--smoke-test --smoke-offscreen) ;;
    *) echo "Expected native or offscreen" >&2; exit 2 ;;
esac
if [[ "$#" -eq 0 ]]; then
    echo "Select at least one scenario" >&2
    exit 2
fi
scenarios=(
    intro inn shop menu equipment item-menu skill-menu
    battle battle-actions battle-menus battle-events battle-rewards battle-transitions battle-defeat
    airship airship-escape airship-escape-affinity
    airship-journey-125 airship-journey-126 airship-journey-127 airship-journey-129
    airship-murder airship-murder-affinity airship-return airship-sky airship-sky-pirate
    airship-free-1 airship-free-2 airship-free-3 airship-free-4
    airship-free-pirate-1 airship-free-pirate-2 airship-free-pirate-3 airship-free-pirate-4
    map-scenes map-passages terrain overlap
    panorama timer font font-colors colors animation-colors actor-graphics actor-names
    message-options dialogue-timing display ui-layers water world-tones map-animations
    map-flashes pictures weather camera looping transitions screen-events quick-transfers
    normal-transfers reserved-transfers async-transitions async-inns gameover return-title
    save-slots load-slots save-music save-npcs save-hero save-vehicles save-camera
    save-pictures save-screen save-weather save-animations
    crystal-2 crystal-18 crystal-45 crystal-74 crystal-79 crystal-91 crystal-98 crystal-111
    crystal-119 crystal-125 crystal-143 crystal-145 crystal-182 crystal-202 crystal-231 crystal-260
)
if [[ "$#" -eq 1 && "$1" == --list ]]; then
    printf '%s\n' "${scenarios[@]}"
    exit 0
fi
requested=()
for scenario in "$@"; do
    if [[ "$scenario" == all ]]; then
        requested+=("${scenarios[@]}")
    else
        requested+=("$scenario")
    fi
done
set -- "${requested[@]}"
binary="$root/target/debug/amnezia"
[[ -x "$binary" ]] || { echo "Build amnezia first" >&2; exit 2; }
if command -v gtimeout >/dev/null; then
    launch=(gtimeout --signal=TERM --kill-after=10)
elif command -v timeout >/dev/null; then
    launch=(timeout --signal=TERM --kill-after=10)
else
    echo "GNU timeout is required to bound graphical test runs" >&2
    exit 2
fi
if [[ "$mode" == native ]] && command -v caffeinate >/dev/null; then
    launch=(caffeinate -dimsu "${launch[@]}")
fi
evidence="$(mktemp -d "${TMPDIR:-/tmp}/amnezia-verification-XXXXXX")"
echo "Evidence: $evidence"
git -C "$root" rev-parse HEAD > "$evidence/revision.txt"
git -C "$root" diff --stat > "$evidence/worktree.txt"
shasum -a 256 "$binary" > "$evidence/binary.sha256"

retain_captures() {
    local name="$1" capture
    mkdir -p "$evidence/$name"
    while IFS= read -r capture; do
        [[ "$capture" == /* && "${capture##*/}" == amnezia-smoke-*.png ]] || continue
        cp -p "$capture" "$evidence/$name/"
    done < <(sed -n 's/.*Screenshot saved to //p' "$evidence/$name.log" | sort -u)
}

run() {
    local name="$1" marker="$2" status=0 limit=180
    case "$name" in airship-return|airship-sky*) limit=300 ;; esac
    shift 2
    echo "Running $mode $name"
    shasum -a 256 --check "$evidence/binary.sha256" > /dev/null
    "${launch[@]}" "$limit" "$binary" "${mode_args[@]}" "$@" > "$evidence/$name.log" 2>&1 || status=$?
    retain_captures "$name"
    if [[ "$status" -ne 0 ]] || ! rg -F -q "smoke scenario '$marker' completed all final checks" "$evidence/$name.log"; then
        tail -35 "$evidence/$name.log" >&2
        echo "FAILED $name (exit $status); evidence retained at $evidence" >&2
        return 1
    fi
    echo "PASS $name"
}

for scenario in "$@"; do
    case "$scenario" in
        airship-free-1|airship-free-2|airship-free-3|airship-free-4)
            run "$scenario" airship-free --smoke-airship-free "--smoke-direction=${scenario##*-}" ;;
        airship-free-pirate-1|airship-free-pirate-2|airship-free-pirate-3|airship-free-pirate-4)
            run "$scenario" airship-free --smoke-airship-free --smoke-airship-pirate "--smoke-direction=${scenario##*-}" ;;
        airship-sky) run "$scenario" "$scenario" --smoke-airship-sky ;;
        airship-sky-pirate) run "$scenario" airship-sky --smoke-airship-sky --smoke-airship-pirate ;;
        airship-return) run "$scenario" "$scenario" --smoke-airship-return ;;
        airship-murder) run "$scenario" airship-journey --smoke-airship-murder ;;
        airship-murder-affinity) run "$scenario" airship-journey --smoke-airship-murder --smoke-airship-affinity ;;
        airship-journey-125|airship-journey-126|airship-journey-127|airship-journey-129)
            run "$scenario" airship-journey --smoke-airship-journey "--smoke-map=${scenario##*-}" ;;
        crystal-*)
            map="${scenario#crystal-}"
            case "$map" in
                2|18|45|74|79|91|98|111|119|125|143|145|182|202|231|260) ;;
                *) echo "Unknown crystal map: $map" >&2; exit 2 ;;
            esac
            slots="$(mktemp -d "${TMPDIR:-/tmp}/amnezia-crystals-XXXXXX")"
            run "$scenario-save" crystals --smoke-crystals "--smoke-map=$map" "--smoke-crystal-dir=$slots"
            run "$scenario-load" crystals --smoke-crystals "--smoke-map=$map" "--smoke-crystal-dir=$slots" --smoke-crystal-resume
            ;;
        airship-escape-affinity)
            run "$scenario" escape --smoke-airship-escape --smoke-airship-affinity ;;
        airship-escape) run "$scenario" escape --smoke-airship-escape ;;
        map-scenes) run "$scenario" map-scenes --smoke-map-scenes ;;
        map-passages) run "$scenario" map-passages --smoke-map-passages ;;
        terrain) run "$scenario" terrain --smoke-terrain ;;
        overlap) run "$scenario" overlap --smoke-overlap ;;
        item-menu) run "$scenario" items --smoke-item-menu ;;
        skill-menu) run "$scenario" skills --smoke-skill-menu ;;
        intro) run "$scenario" intro ;;
        inn|shop|menu|equipment|battle|battle-actions|battle-menus|battle-events|battle-rewards|battle-transitions|battle-defeat|airship|panorama|timer|font|font-colors|colors|animation-colors|actor-graphics|actor-names|message-options|dialogue-timing|display|ui-layers|water|world-tones|map-animations|map-flashes|pictures|weather|camera|looping|transitions|screen-events|quick-transfers|normal-transfers|reserved-transfers|async-transitions|async-inns|gameover|return-title|save-slots|load-slots|save-music|save-npcs|save-hero|save-vehicles|save-camera|save-pictures|save-screen|save-weather|save-animations)
            run "$scenario" "$scenario" "--smoke-$scenario" ;;
        *) echo "Unknown scenario: $scenario" >&2; exit 2 ;;
    esac
done
echo "All requested $mode scenarios passed; evidence: $evidence"
