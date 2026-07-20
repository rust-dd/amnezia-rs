export const meta = {
  name: 'amnezia-fidelity-audit',
  description: 'Deep RM2000 fidelity audit of Amnezia across all non-battle/animation systems vs original data + local EasyRPG',
  phases: [
    { title: 'Audit' },
    { title: 'Verify' },
    { title: 'Synthesize' },
  ],
}

const P = 'reference/easyrpg-player/src'
const L = 'reference/liblcf/src'

const SYSTEMS = [
  { key: 'maps-passability', ours: 'amnezia/src/world/{tiles,movement,render}.rs; amnezia-data Map/Chipset; amnezia-convert maps/chipsets', ref: `${P}/game_map.cpp, game_player.cpp, game_character.cpp`, focus: 'tile passability (lower+upper layers, the star/above-hero attr, counter, bush, ladder), terrain ids, map bounds/scroll clamp, animated water tiles, layer draw order' },
  { key: 'movement-events', ours: 'amnezia/src/world/movement.rs, player.rs; event movement/render', ref: `${P}/game_character.cpp, game_player.cpp, game_event.cpp`, focus: 'step speed/frequency, event autonomous move (random/approach/custom route), event trigger types (action/touch/collision/auto/parallel), through flag, hero-event collision, facing on move' },
  { key: 'interpreter-opcodes', ours: 'amnezia/src/interpreter/{opcodes,commands}.rs, interpreter.rs', ref: `${P}/game_interpreter.cpp, game_interpreter_map.cpp`, focus: 'which event commands remain UNHANDLED vs EasyRPG, and correctness of handled ones: conditional branch, loops/labels/break, ChangeSwitch/Variable ops, ChangeItems/Gold/Party/Level/Skills/Equipment/HP/SP, Wait, MoveRoute, CallEvent, key input, conditions' },
  { key: 'dialogue-messages', ours: 'amnezia/src/dialogue.rs', ref: `${P}/window_message.cpp, game_message.cpp`, focus: 'message box position (top/mid/bottom), face graphic side, control codes (\\N \\V \\C \\S \\$ \\! \\. \\| \\^ \\> \\< \\_ \\\\), letter-by-letter speed, choices window, number input, word-wrap, continue arrow, transparent frame' },
  { key: 'shop', ours: 'amnezia/src/shop.rs', ref: `${P}/scene_shop.cpp, window_shop.cpp, window_shopbuy.cpp`, focus: 'buy/sell flow, sell price = half buy, stock/qty limits (99), gold display, shop type variants (buy-only/sell-only), greeting/purchase/regret messages' },
  { key: 'save-load', ours: 'amnezia/src/save.rs', ref: `${P}/scene_save.cpp, scene_load.cpp, sprite_battler? no; game_system + lcf RPG::Save`, focus: 'what is persisted (party roster, vitals HP/SP, inventory, gold, switches, variables, map id + position + facing, playtime, BGM), our single-slot deviation, load restoring all of it, save availability' },
  { key: 'title-newgame', ours: 'amnezia/src/title.rs', ref: `${P}/scene_title.cpp`, focus: 'title menu (New Game / Continue / Shutdown), title graphic + BGM, cursor, disabling Continue with no save, transition into the first map at the party start, the intro sequence' },
  { key: 'audio-bgm-se', ours: 'grep AudioRequest / bgm across amnezia/src; midi module', ref: `${P}/audio.cpp, game_system.cpp`, focus: 'PlayBGM change + fade + loop, Memorize/Restore BGM, PlaySE, volume/tempo/balance params, map-defined BGM vs event BGM, battle BGM/victory ME, silence sentinel' },
  { key: 'screen-effects', ours: 'amnezia/src/screenfx.rs', ref: `${P}/game_screen.cpp`, focus: 'tint screen (RGB+saturation, duration/interp), flash (color+power+duration), shake (power/speed/duration/continuous), weather (rain/snow/fog/sand + strength), fade erase/show, and the opcodes driving them (10630? 11020 tint, 11030 flash, 11040 shake, 11070 weather)' },
  { key: 'pictures', ours: 'amnezia/src/picture.rs', ref: `${P}/game_pictures.cpp, sprite_picture.cpp`, focus: 'ShowPicture (id, position, scroll-with-map flag, zoom, tone, transparency, layer/z), MovePicture (interpolated over duration), ErasePicture, pin vs map-relative coords' },
  { key: 'switches-variables', ours: 'amnezia/src/state.rs (switches/variables) or grep', ref: `${P}/game_switches.cpp, game_variables.cpp, game_interpreter.cpp control ops`, focus: 'switch/variable id ranges + default 0/false, batch range ops, variable operand kinds (constant/var/varvar/random/item count/actor param/event/other), operations (set/add/sub/mul/div/mod), persistence across save' },
  { key: 'party-progression', ours: 'amnezia/src/progression.rs, state.rs Party, gamedata actors', ref: `${P}/game_party.cpp, game_actor.cpp`, focus: 'exp curve exactness, level-up stat gains + learned skills + level-up message, add/remove actor, max 4 party, starting members, gold cap, item-count cap' },
  { key: 'teleport-transfer', ours: 'amnezia/src/teleport.rs', ref: `${P}/game_player.cpp (PerformTeleport), scene_map transitions`, focus: 'transfer fade-out/in timing + color, destination facing param, same-map vs cross-map, screen scroll, hero visibility during transfer, teleport from event vs command' },
  { key: 'encounters', ours: 'grep encounter/random battle in amnezia/src/world', ref: `${P}/game_map.cpp (encounter steps), game_player.cpp`, focus: 'random-encounter step counter + rate, per-terrain troop tables, encounter regions/areas, the RM2000 steps-until-battle formula + variance, disabling encounters' },
  { key: 'i18n-text', ours: 'amnezia/src/i18n.rs, assets/i18n/en.ron', ref: 'original strings in original/ + assets data', focus: 'EN coverage vs HU (menus/items/skills/dialogue), control-code preservation in translations, missing/empty keys, term/name consistency, the F2 toggle correctness' },
]

const FINDINGS = { type: 'object', properties: { findings: { type: 'array', items: { type: 'object', properties: { title: { type: 'string' }, ours: { type: 'string', description: 'our current behavior + file:line' }, reference: { type: 'string', description: 'RM2000/EasyRPG behavior + cited file/function' }, impact: { type: 'string', description: 'user-visible effect' }, severity: { type: 'string', enum: ['high', 'medium', 'low'] }, fix: { type: 'string' } }, required: ['title', 'ours', 'reference', 'severity'] } } }, required: ['findings'] }

const VERDICT = { type: 'object', properties: { real: { type: 'boolean' }, reason: { type: 'string' } }, required: ['real', 'reason'] }

phase('Audit')
const results = await pipeline(
  SYSTEMS,
  (sys) => agent(`INVESTIGATION ONLY — do NOT edit any file, do NOT run git. Deep RM2000 (2000, not 2003) fidelity audit of the "${sys.key}" system in the Amnezia remake at /Users/danixx/Desktop/amnezia-rs. Compare OUR implementation against the ORIGINAL game data (original/, assets/) and the LOCAL EasyRPG source at ${P} and liblcf at ${L} — read those files directly with Grep/Read, do NOT use the web.
OUR CODE: ${sys.ours}
EASYRPG REFERENCE: ${sys.ref}
FOCUS: ${sys.focus}
Grep/read our modules and the cited EasyRPG files. List EVERY deviation from faithful RM2000/2000 behavior; SKIP anything we already do faithfully. Each finding: a specific title; ours (behavior + our file:line); reference (EasyRPG file/function + the correct behavior); impact (user-visible); severity high/medium/low; a concrete fix sketch. Be exhaustive and concrete.`, { label: `audit:${sys.key}`, phase: 'Audit', schema: FINDINGS }),
  (res, sys) => parallel((res && res.findings ? res.findings : []).map((f) => () =>
    agent(`Adversarially verify this claimed RM2000 fidelity gap in the Amnezia remake's "${sys.key}" system. Read our code AND the LOCAL EasyRPG source at ${P} (liblcf at ${L}) to confirm — no web. Is this a REAL deviation from faithful RM2000/2000, or is our behavior actually correct/acceptable (or already handled)? Default real=false if uncertain or if ours is faithful.
CLAIM: ${f.title}
OURS: ${f.ours}
REFERENCE: ${f.reference}
FIX: ${f.fix || 'n/a'}`, { label: `verify:${sys.key}`, phase: 'Verify', schema: VERDICT })
      .then((v) => ({ system: sys.key, title: f.title, ours: f.ours, reference: f.reference, impact: f.impact, severity: f.severity, fix: f.fix, real: v ? v.real : false, reason: v ? v.reason : 'verifier died' }))
  )),
)
const all = results.flat().filter(Boolean)
const confirmed = all.filter((f) => f.real)
log(`${SYSTEMS.length} systems audited • ${all.length} findings • ${confirmed.length} confirmed real`)

phase('Synthesize')
const report = await agent(`Synthesize these CONFIRMED RM2000 fidelity gaps for the Amnezia remake into ONE prioritized, deduplicated action list. Group by system; within each, order high→low severity; each item = a one-line title + the fix sketch + the file(s) to touch. End with a "TOP 15 highest-impact fixes overall" ranked section. Confirmed findings (JSON):
${JSON.stringify(confirmed)}`, { phase: 'Synthesize' })

return { systems: SYSTEMS.length, findings: all.length, confirmed: confirmed.length, list: confirmed, report }
