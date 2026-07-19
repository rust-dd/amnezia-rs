//! Battle resolution: apply the agility-ordered turn queue to the live
//! [`Battle`], one action at a time — physical hits, skill casts, item heals,
//! defence, and enemy attacks — then check for the end, tally rewards, and roll a
//! flee. Extends [`Battle`] with a second `impl` block so the turn-flow half stays
//! in [`super::model`]; the damage numbers themselves come from [`super::logic`].

use super::BattleOutcome;
use super::logic;
use super::model::{
    Action, Battle, BattleSe, Command, Dying, NumberKind, PendingAnim, PendingNumber, Phase,
    Source, Step, rng_next,
};
use crate::i18n;
use amnezia_data::SkillDef;

/// RM2000 front-view draws no party sprites, so a hit a member takes animates at
/// the bottom-centre party area — just below the screen centre (y grows down).
const PARTY_ANIM_Y: f32 = 80.0;

/// Horizontal spacing between adjacent members' incoming-hit animations, so a
/// multi-member party doesn't stack every hit at the exact centre.
const PARTY_ANIM_SPREAD: f32 = 16.0;

/// Seconds a slain foe blinks and fades out before it is cleared (RM2000
/// `SetDeathTimer(36)` counted down at 60 fps).
const DEATH_SECS: f32 = 36.0 / 60.0;

/// Seconds a self-destructing foe zoom-fades before it is cleared (RM2000
/// `SetExplodeTimer(20)` at 60 fps) — a shorter, punchier burst than a plain death.
const EXPLODE_SECS: f32 = 20.0 / 60.0;

/// The outcome of a party member's weapon strike: a clean miss, or a landed hit
/// carrying the damage dealt and whether it critical'd (for the log line).
#[derive(Clone, Copy)]
enum Strike {
    Miss,
    Hit { dmg: i32, crit: bool },
}

/// The floating-number text and colour for a landed blow of `dmg`: the digits in
/// white for a real hit, or a pale "0" for a blocked/immune blow (RM2000 pops "0"
/// on an undamaged hit — distinct from a dodge, which pops "Miss").
fn number_for(dmg: i32) -> (String, NumberKind) {
    if dmg > 0 {
        (dmg.to_string(), NumberKind::Damage)
    } else {
        ("0".to_string(), NumberKind::Miss)
    }
}

impl Battle {
    /// Advance the resolution by one beat and report whether more remains. A
    /// pending sub-step (a further target of a multi-target cast, or a critical's
    /// damage beat) is drained first, one per call, so those beats stagger across
    /// ticks; otherwise the next queued action is applied (skipping a fainted
    /// actor, retargeting a dead target) and its log line appended. Returns
    /// `false` only once both the step buffer and the queue are spent.
    pub fn resolve_next(&mut self) -> bool {
        if let Some(step) = self.steps.pop_front() {
            self.run_step(step);
            return true;
        }
        let Some(&action) = self.queue.get(self.queue_at) else {
            return false;
        };
        self.queue_at += 1;
        if self.source_alive(action.source) {
            // RM2000 applies an HP-changing state (poison drain, regen) at the
            // start of the battler's turn, before it acts; a drain that fells the
            // battler cancels its action through the HP-based death path.
            self.tick_state_hp(action.source);
            if self.source_alive(action.source) {
                self.apply(action);
            }
        }
        true
    }

    /// Resolve one deferred [`Step`] of the action in progress: the next target of
    /// a multi-target cast (rolled now, so the RNG draw order matches a single-tick
    /// cast), or the damage beat that follows a critical's "Kritikus!" line. Each
    /// pushes its own log line(s), so a multi-target cast reads as one message per
    /// target and a felled target starts its own death-hold before the next beat.
    fn run_step(&mut self, step: Step) {
        match step {
            Step::HitEnemy { pi, ti, skill_id } => {
                if !self.enemies.get(ti).is_some_and(|e| e.alive()) {
                    return;
                }
                if let Some(skill) = self.skills.iter().find(|s| s.id == skill_id).cloned() {
                    let lines = self.skill_hit_enemy(pi, ti, &skill);
                    self.log.extend(lines);
                }
            }
            Step::HealAlly { pi, ti, skill_id } => {
                if !self.members.get(ti).is_some_and(|m| m.alive()) {
                    return;
                }
                if let Some(skill) = self.skills.iter().find(|s| s.id == skill_id).cloned() {
                    let lines = self.skill_heal_ally(pi, ti, &skill);
                    self.log.extend(lines);
                }
            }
            Step::CritDamage { pi, ti, dmg } => {
                self.land_strike(ti, dmg);
                let line = format!(
                    "{} rácsap: {} -{}",
                    self.members[pi].name, self.enemies[ti].name, dmg
                );
                self.log.push(line);
            }
            Step::StrikeImpact {
                pi,
                ti,
                dmg,
                crit,
                miss,
            } => {
                let outcome = if miss {
                    Strike::Miss
                } else {
                    Strike::Hit { dmg, crit }
                };
                self.resolve_strike_impact(pi, ti, outcome);
            }
            Step::CastSkill {
                pi,
                skill_id,
                target,
            } => {
                // Re-run the cast with the animation suppressed (it was queued when
                // the action began); its effect, numbers, and RNG resolve now.
                self.suppress_anim = true;
                let line = self.cast_skill(pi, skill_id, target);
                self.suppress_anim = false;
                if let Some(line) = line {
                    self.log.push(line);
                }
            }
            Step::EnemyCast {
                ei,
                skill_id,
                target,
            } => {
                self.suppress_anim = true;
                let line = self.enemy_cast(ei, skill_id, target);
                self.suppress_anim = false;
                if let Some(line) = line {
                    self.log.push(line);
                }
            }
        }
    }

    fn source_alive(&self, source: Source) -> bool {
        match source {
            Source::Party(i) => self.members.get(i).is_some_and(|f| f.alive()),
            Source::Enemy(i) => self.enemies.get(i).is_some_and(|e| e.alive()),
        }
    }

    /// Apply every active HP-changing state to `source` at the start of its turn
    /// (RM2000 timing, before it acts): a type-0 state drains and a type-1 state
    /// regenerates [`logic::state_hp_delta`]'s per-turn amount, a type-2 (and any
    /// unconfigured) state does nothing. A drain floors HP at 0 — from there the
    /// battler is defeated by the existing HP-based death path — and a regen caps
    /// at max HP. Each change is logged in the state-line style. Map/field HP
    /// change (`hp_change_map_*`) is deliberately skipped: it needs persistent
    /// overworld states, which battle state does not carry.
    fn tick_state_hp(&mut self, source: Source) {
        let (active, max_hp): (Vec<u32>, i32) = match source {
            Source::Party(i) => (
                self.members[i].states.iter().map(|&(id, _)| id).collect(),
                self.members[i].max_hp,
            ),
            Source::Enemy(i) => (
                self.enemies[i].states.iter().map(|&(id, _)| id).collect(),
                self.enemies[i].max_hp,
            ),
        };
        let mut changes: Vec<(i32, String)> = Vec::new();
        for sid in active {
            if let Some(def) = self.states.iter().find(|d| d.id == sid) {
                let delta = logic::state_hp_delta(def, max_hp);
                if delta != 0 {
                    changes.push((delta, def.name.clone()));
                }
            }
        }
        for (delta, state_name) in changes {
            let name = match source {
                Source::Party(i) => {
                    let f = &mut self.members[i];
                    f.hp = (f.hp + delta).clamp(0, max_hp);
                    f.name.clone()
                }
                Source::Enemy(i) => {
                    let e = &mut self.enemies[i];
                    e.hp = (e.hp + delta).clamp(0, max_hp);
                    e.name.clone()
                }
            };
            self.log.push(if delta < 0 {
                format!("{name}: {state_name} -{}", -delta)
            } else {
                format!("{name}: {state_name} +{delta}")
            });
        }
        // A drain that emptied a foe's HP starts its death-out like any other kill.
        if let Source::Enemy(i) = source {
            self.start_foe_death(i, false);
        }
    }

    fn apply(&mut self, action: Action) {
        let line = match (action.source, action.kind) {
            (Source::Party(pi), Command::Attack { target }) => {
                if logic::worst_restriction(&self.members[pi].states, &self.states) == 3 {
                    let Some(ti) = self.retarget_ally(pi, target) else {
                        return;
                    };
                    let base = logic::physical_damage(
                        self.members[pi].stats.attack,
                        self.members[ti].stats.defense,
                    );
                    let dmg = self.hit_member(ti, base, 4);
                    format!(
                        "{} zavartan lesújt: {} -{}",
                        self.members[pi].name, self.members[ti].name, dmg
                    )
                } else {
                    let Some(ti) = self.retarget_enemy(target) else {
                        return;
                    };
                    let outcome = self.plan_strike(pi, ti);
                    // RM2000 plays the swing animation and waits for it before the
                    // damage. When the strike animates, defer its impact behind the
                    // animation hold so the number lands as the swing finishes; a
                    // member with no attack animation has nothing to wait for and
                    // applies it at once.
                    if self.members[pi].attack_animation != 0 {
                        let (dmg, crit, miss) = match outcome {
                            Strike::Miss => (0, false, true),
                            Strike::Hit { dmg, crit } => (dmg, crit, false),
                        };
                        self.steps.push_back(Step::StrikeImpact {
                            pi,
                            ti,
                            dmg,
                            crit,
                            miss,
                        });
                        self.begin_anim_hold();
                    } else {
                        self.resolve_strike_impact(pi, ti, outcome);
                    }
                    return;
                }
            }
            (Source::Party(pi), Command::Skill { skill_id, target }) => {
                // A skill with a battle animation waits for it before its effect
                // lands (RM2000). Queue the animation up front, then defer the cast
                // behind the hold so its numbers land as the cast finishes; an
                // animation-less skill resolves at once.
                let Some(skill) = self.skills.iter().find(|s| s.id == skill_id).cloned() else {
                    return;
                };
                let anchors = self.skill_anim_anchors(pi, &skill, target);
                if skill.animation_id != 0 && !anchors.is_empty() {
                    self.push_anim(skill.animation_id, anchors);
                    self.steps.push_back(Step::CastSkill {
                        pi,
                        skill_id,
                        target,
                    });
                    self.begin_anim_hold();
                    return;
                }
                match self.cast_skill(pi, skill_id, target) {
                    Some(line) => line,
                    None => return,
                }
            }
            (Source::Party(pi), Command::Item { item_id, target }) => {
                self.apply_item(pi, item_id, target)
            }
            (Source::Party(pi), Command::Defend) => {
                self.members[pi].defending = true;
                format!("{} védekezik", self.members[pi].name)
            }
            (Source::Party(pi), Command::Nothing) => {
                format!("{} tétovázik", self.members[pi].name)
            }
            (Source::Enemy(ei), Command::Attack { target }) => {
                if logic::worst_restriction(&self.enemies[ei].states, &self.states) == 3 {
                    let Some(ti) = self.retarget_other_enemy(ei, target) else {
                        return;
                    };
                    let base = logic::physical_damage(
                        self.enemies[ei].stats.attack,
                        self.enemies[ti].stats.defense,
                    );
                    let dmg = self.hit_enemy(ti, base, 4);
                    format!(
                        "{} zavartan lesújt: {} -{}",
                        self.enemies[ei].name, self.enemies[ti].name, dmg
                    )
                } else {
                    let Some(ti) = self.retarget_member(target) else {
                        return;
                    };
                    let enemy = self.enemies[ei].name.clone();
                    let member = self.members[ti].name.clone();
                    match self.enemy_strike_member(ei, ti) {
                        Some(dmg) => format!("{enemy} támad: {member} -{dmg}"),
                        None => format!("{enemy} támad: {member} elkerülte"),
                    }
                }
            }
            (Source::Enemy(ei), Command::Skill { skill_id, target }) => {
                // As for a party cast: an animated enemy skill waits for its
                // animation before the effect lands; queue it, then defer.
                let Some(skill) = self.skills.iter().find(|s| s.id == skill_id).cloned() else {
                    return;
                };
                let anchors = self.enemy_skill_anim_anchors(ei, &skill, target);
                if skill.animation_id != 0 && !anchors.is_empty() {
                    self.push_anim(skill.animation_id, anchors);
                    self.steps.push_back(Step::EnemyCast {
                        ei,
                        skill_id,
                        target,
                    });
                    self.begin_anim_hold();
                    return;
                }
                match self.enemy_cast(ei, skill_id, target) {
                    Some(line) => line,
                    None => return,
                }
            }
            (Source::Enemy(ei), Command::DoubleAttack { target }) => {
                let Some(ti) = self.retarget_member(target) else {
                    return;
                };
                let d1 = self.enemy_strike_member(ei, ti);
                let d2 = self.enemy_strike_member(ei, ti);
                let enemy = self.enemies[ei].name.clone();
                let member = self.members[ti].name.clone();
                let show =
                    |d: Option<i32>| d.map_or_else(|| "elkerülte".to_string(), |v| format!("-{v}"));
                format!("{enemy} kétszer támad: {member} {}, {}", show(d1), show(d2))
            }
            (Source::Enemy(ei), Command::Defend) => {
                self.enemies[ei].defending = true;
                format!("{} védekezik", self.enemies[ei].name)
            }
            (Source::Enemy(ei), Command::SelfDestruct) => {
                let atk = self.enemies[ei].stats.attack as i32;
                let name = self.enemies[ei].name.clone();
                for ti in self.living_members() {
                    // EasyRPG `CalcSelfDestructEffect`: max(0, atk - def/2) against
                    // each member's own defence, then the var=4 spread.
                    let base = (atk - self.members[ti].stats.defense as i32 / 2).max(0);
                    self.hit_member(ti, base, 4);
                }
                self.enemies[ei].hp = 0;
                self.start_foe_death(ei, true);
                format!("{name} felrobban!")
            }
            (Source::Enemy(ei), Command::Escape) => {
                self.enemies[ei].fled = true;
                format!("{} elmenekül", self.enemies[ei].name)
            }
            (Source::Enemy(ei), Command::Charge) => {
                self.enemies[ei].charging = true;
                format!("{} erőt gyűjt", self.enemies[ei].name)
            }
            (Source::Enemy(ei), Command::Nothing) => {
                format!("{} tétovázik", self.enemies[ei].name)
            }
            _ => return,
        };
        self.log.push(line);
    }

    /// Queue `anim_id` on `targets` (RM2000 screen offsets from centre, y
    /// downward) for `battle.rs` to play as this tick resolves — one queued
    /// animation for the whole cast, so its sound fires once while its cells and
    /// flashes reach every target. A `0` id (no animation) or an empty target list
    /// is skipped, so an attacker with neither a weapon nor an unarmed animation
    /// plays nothing rather than a stray effect.
    fn push_anim(&mut self, anim_id: u32, targets: Vec<(f32, f32)>) {
        if anim_id != 0 && !targets.is_empty() {
            self.pending_anims.push(PendingAnim { anim_id, targets });
        }
    }

    /// The horizontal screen offset for member `ti`'s incoming-hit animation:
    /// centred on 0 and fanned out a little by member index (see
    /// [`PARTY_ANIM_SPREAD`]), since the party isn't drawn in front view.
    fn party_anim_x(&self, ti: usize) -> f32 {
        let count = self.members.len().max(1) as f32;
        (ti as f32 - (count - 1.0) / 2.0) * PARTY_ANIM_SPREAD
    }

    /// The screen offset for foe `ti`'s battle animation: its backdrop placement
    /// re-centred on the RM2000 320×240 screen (minus the 160×120 half-extent, y
    /// downward), matching the animation player's coordinates.
    fn foe_anim_pos(&self, ti: usize) -> (f32, f32) {
        (
            self.enemies[ti].x as f32 - 160.0,
            self.enemies[ti].y as f32 - 120.0,
        )
    }

    /// Queue a floating number to pop on a battler at `pos` as this action
    /// resolves; `battle::floaters` spawns and rises it. Bevy-free.
    fn push_number(&mut self, pos: (f32, f32), text: String, kind: NumberKind) {
        self.pending_numbers.push(PendingNumber { pos, text, kind });
    }

    /// Register a landed blow of `dmg` on foe `ti`: play the enemy-damaged SE, pop
    /// its damage number, owe it a guaranteed whitening blink (RM2000 blinks a
    /// struck sprite every hit, animation-flash or not), and start its death-out if
    /// the blow felled it. The SE fires on any landed blow (even a blocked 0), like
    /// EasyRPG's damage-message substate; a felled foe then adds the kill SE via
    /// [`Battle::start_foe_death`].
    fn after_foe_hit(&mut self, ti: usize, dmg: i32) {
        let pos = self.foe_anim_pos(ti);
        self.pending_se.push(BattleSe::EnemyDamaged);
        let (text, kind) = number_for(dmg);
        self.push_number(pos, text, kind);
        self.pending_blinks.push(pos);
        self.start_foe_death(ti, false);
    }

    /// Register a landed blow of `dmg` on member `ti`: play the actor-damaged SE
    /// and pop its damage number at the party slot. Party members have no
    /// front-view sprite, so the number is their whole visual feedback (no blink,
    /// no death-out).
    fn after_member_hit(&mut self, ti: usize, dmg: i32) {
        let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
        self.pending_se.push(BattleSe::ActorDamaged);
        let (text, kind) = number_for(dmg);
        self.push_number(pos, text, kind);
    }

    /// Start foe `ti`'s death-out if it was just reduced to 0 HP and isn't already
    /// leaving: a zoom-fade explosion for a self-destruct (`explode`), else the
    /// RM2000 blink-and-fade. A fled foe (it kept its HP) and an already-dying foe
    /// are left alone. `resolve_tick` then holds until it elapses.
    fn start_foe_death(&mut self, ti: usize, explode: bool) {
        let foe = &mut self.enemies[ti];
        if foe.hp <= 0 && !foe.fled && foe.dying.is_none() {
            foe.dying = Some(Dying {
                elapsed: 0.0,
                secs: if explode { EXPLODE_SECS } else { DEATH_SECS },
                explode,
            });
            // The kill SE fires once, guarded by the same transition that starts
            // the death-out (EasyRPG `ProcessBattleActionDeath`).
            self.pending_se.push(BattleSe::EnemyDefeated);
        }
    }

    /// Whether any foe is mid death-out; `resolve_tick` holds the step while so.
    pub(super) fn death_in_progress(&self) -> bool {
        self.enemies
            .iter()
            .any(|e| e.dying.as_ref().is_some_and(|d| d.elapsed < d.secs))
    }

    /// Advance every in-progress death-out by `dt` real seconds (driven each frame
    /// by `battle::scene`), clamped at its length so [`Battle::death_in_progress`]
    /// eventually clears and resolution resumes.
    pub(super) fn advance_deaths(&mut self, dt: f32) {
        for foe in &mut self.enemies {
            if let Some(d) = &mut foe.dying
                && d.elapsed < d.secs
            {
                d.elapsed = (d.elapsed + dt).min(d.secs);
            }
        }
    }

    /// Roll a party member's weapon strike on enemy `ti` to its final damage and
    /// critical flag, in RM2000 order — an agility-adjusted to-hit roll (bare hands
    /// default 90%), then on a hit the weapon's element against the foe's
    /// resistance ranks, a critical that triples, the `var=4` variance, and the
    /// defending-foe halving — *without applying it or showing anything yet*. The
    /// attack animation is queued on the struck foe up front (the swing shows
    /// whether the blow lands); both the "Miss" pop and the landing are deferred
    /// to [`Battle::resolve_strike_impact`], which runs only once the animation
    /// has played. The RNG draw order is identical to a single-shot strike.
    fn plan_strike(&mut self, pi: usize, ti: usize) -> Strike {
        let anim = self.members[pi].attack_animation;
        self.push_anim(anim, vec![self.foe_anim_pos(ti)]);
        let base = logic::physical_damage(
            self.members[pi].stats.attack,
            self.enemies[ti].stats.defense,
        );
        // A foe that cannot act (asleep/paralyzed) is struck with certainty
        // (EasyRPG `CalcNormalAttackToHit` returns 100 vs a do-nothing target).
        let can_act = logic::worst_restriction(&self.enemies[ti].states, &self.states) != 1;
        let hit = logic::to_hit_vs(
            logic::effective_hit(self.members[pi].weapon_hit),
            self.members[pi].stats.agility,
            self.enemies[ti].stats.agility,
            can_act,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
            return Strike::Miss;
        }
        let element = self.members[pi].weapon_element.unwrap_or(0);
        let base = logic::elemental_damage(
            base,
            element,
            &self.enemies[ti].attribute_ranks,
            &self.attributes,
        );
        let crit = ((rng_next(&mut self.rng) % 100) as u32) < self.members[pi].weapon_crit;
        let base = if crit {
            logic::critical_damage(base)
        } else {
            base
        };
        let roll = rng_next(&mut self.rng);
        let mut dmg = logic::variance_adjust(base, 4, roll).max(0);
        // A defending foe halves the final result — plain `dmg/2`, no floor, so a
        // foe and a member Defend behave alike (EasyRPG `AdjustDamageForDefend`),
        // applied after element/crit/variance.
        if self.enemies[ti].defending {
            dmg = logic::defended(dmg);
        }
        Strike::Hit { dmg, crit }
    }

    /// Land a planned strike of `dmg` on enemy `ti`: subtract the HP, roll its
    /// states' damage wear-off, and pop the damage number, whitening blink, and
    /// death-out (see [`Battle::after_foe_hit`]).
    fn land_strike(&mut self, ti: usize, dmg: i32) {
        self.enemies[ti].hp -= dmg;
        self.release_states_on_enemy(ti);
        self.after_foe_hit(ti, dmg);
    }

    /// Apply a member's planned strike `outcome` on foe `ti` once its swing
    /// animation has played (RM2000 `ProcessBattleActionApply`/`Damage`): a miss
    /// pops the dodge SE and "Miss" number, a critical announces on its own beat
    /// and lands its precomputed `dmg` on the next tick ([`Step::CritDamage`]),
    /// and a plain hit lands at once — each logging its line. Draws no RNG, so the
    /// order is unchanged whether this runs inline or deferred behind the hold.
    fn resolve_strike_impact(&mut self, pi: usize, ti: usize, outcome: Strike) {
        let member = self.members[pi].name.clone();
        let enemy = self.enemies[ti].name.clone();
        match outcome {
            Strike::Miss => {
                let pos = self.foe_anim_pos(ti);
                self.pending_se.push(BattleSe::Dodge);
                self.push_number(pos, "Miss".to_string(), NumberKind::Miss);
                self.log.push(format!("{member} rácsap: {enemy} elkerülte"));
            }
            Strike::Hit { dmg, crit: false } => {
                self.land_strike(ti, dmg);
                self.log.push(format!("{member} rácsap: {enemy} -{dmg}"));
            }
            Strike::Hit { dmg, crit: true } => {
                // RM2000 `ProcessBattleActionCritical`: announce the critical on
                // its own beat, then land the (already rolled) blow next tick.
                self.steps.push_back(Step::CritDamage { pi, ti, dmg });
                self.log.push("Kritikus!".to_string());
            }
        }
    }

    /// Resolve a member's weapon strike on enemy `ti` in one shot: plan it, then
    /// apply its impact at once. The [`Battle::apply`] attack path defers the
    /// impact behind the swing animation instead, so this atomic form only serves
    /// the strike unit tests.
    #[cfg(test)]
    fn strike_enemy(&mut self, pi: usize, ti: usize) -> Strike {
        let outcome = self.plan_strike(pi, ti);
        self.resolve_strike_impact(pi, ti, outcome);
        outcome
    }

    /// Apply `base` damage to enemy `ti` with `var` variance (4 for a physical
    /// blow, the skill's variance for a cast), one draw per hit, then the plain
    /// defending-foe halving (no floor, like a member Defend). Returns the damage
    /// dealt.
    fn hit_enemy(&mut self, ti: usize, base: i32, var: i32) -> i32 {
        let roll = rng_next(&mut self.rng);
        let mut dmg = logic::variance_adjust(base, var, roll).max(0);
        if self.enemies[ti].defending {
            dmg = logic::defended(dmg);
        }
        self.enemies[ti].hp -= dmg;
        self.release_states_on_enemy(ti);
        self.after_foe_hit(ti, dmg);
        dmg
    }

    /// Apply `base` damage to member `ti` with `var` variance (4 for a physical
    /// blow, the skill's variance for a cast), one draw per hit, then the member's
    /// own defend halving. Returns the damage dealt.
    fn hit_member(&mut self, ti: usize, base: i32, var: i32) -> i32 {
        let roll = rng_next(&mut self.rng);
        let mut dmg = logic::variance_adjust(base, var, roll).max(0);
        if self.members[ti].defending {
            dmg = logic::defended(dmg);
        }
        self.members[ti].hp -= dmg;
        self.release_states_on_member(ti);
        self.after_member_hit(ti, dmg);
        dmg
    }

    /// One enemy `ei` physical strike on member `ti`: an agility-adjusted to-hit
    /// roll off the RM2000 90% bare-hands base that returns `None` on a miss, else
    /// the dealt damage via `hit_member` (its variance and the member's own defend
    /// halving). A pending charge-up doubles the blow; it is spent on the swing
    /// whether or not the blow lands, so the foe's next strike is normal again. An
    /// rpg2k enemy normal attack plays no animation, so none is queued here (a
    /// weapon/unarmed animation on a party strike is a member-side concern).
    fn enemy_strike_member(&mut self, ei: usize, ti: usize) -> Option<i32> {
        let charged = self.enemies[ei].charging;
        self.enemies[ei].charging = false;
        // A member that cannot act is struck with certainty (EasyRPG
        // `CalcNormalAttackToHit` returns 100 vs a do-nothing target).
        let can_act = logic::worst_restriction(&self.members[ti].states, &self.states) != 1;
        let hit = logic::to_hit_vs(
            logic::effective_hit(0),
            self.enemies[ei].stats.agility,
            self.members[ti].stats.agility,
            can_act,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
            let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
            self.pending_se.push(BattleSe::Dodge);
            self.push_number(pos, "Miss".to_string(), NumberKind::Miss);
            return None;
        }
        let mut base = logic::physical_damage(
            self.enemies[ei].stats.attack,
            self.members[ti].stats.defense,
        );
        if charged {
            base *= 2;
        }
        Some(self.hit_member(ti, base, 4))
    }

    /// Resolve member `pi`'s cast of skill `skill_id` at `target`: deduct SP, then
    /// apply the scoped effect — an elemental attack that may inflict states
    /// (enemy scope 0/1), or a heal that may cure them (ally scope 2/3/4). Returns
    /// the joined log line(s), or `None` for an unknown skill id (no effect).
    fn cast_skill(&mut self, pi: usize, skill_id: u32, target: usize) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id).cloned()?;
        self.members[pi].sp = (self.members[pi].sp - skill.sp_cost as i32).max(0);
        // Queue the skill's battle animation once, over every target it resolves
        // against, so its sound plays once for the cast while its cells and
        // flashes land on each target. Suppressed when the cast is replayed behind
        // its animation hold (`apply` already queued it) so it is never queued
        // twice; the per-target effect helpers below never queue it themselves.
        if !self.suppress_anim {
            let anchors = self.skill_anim_anchors(pi, &skill, target);
            self.push_anim(skill.animation_id, anchors);
        }
        let mut lines: Vec<String> = Vec::new();
        match skill.scope {
            1 => {
                let foes = self.living_enemies();
                // Resolve the first target now and stagger the rest one per tick,
                // so an all-enemy cast's damage numbers appear in sequence rather
                // than all at once. The RNG draw order is unchanged (targets still
                // resolve in living-enemy order).
                let mut targets = foes.into_iter();
                if let Some(first) = targets.next() {
                    lines.extend(self.skill_hit_enemy(pi, first, &skill));
                }
                for ti in targets {
                    self.steps.push_back(Step::HitEnemy {
                        pi,
                        ti,
                        skill_id: skill.id,
                    });
                }
            }
            2 => {
                lines.extend(self.skill_heal_ally(pi, pi, &skill));
            }
            3 => {
                if self.members.get(target).is_some_and(|m| m.alive()) {
                    lines.extend(self.skill_heal_ally(pi, target, &skill));
                }
            }
            4 => {
                let allies = self.living_members();
                // As for scope 1, heal the first ally now and stagger the rest one
                // per tick so the restore numbers appear in sequence.
                let mut targets = allies.into_iter();
                if let Some(first) = targets.next() {
                    lines.extend(self.skill_heal_ally(pi, first, &skill));
                }
                for ti in targets {
                    self.steps.push_back(Step::HealAlly {
                        pi,
                        ti,
                        skill_id: skill.id,
                    });
                }
            }
            _ => {
                if let Some(ti) = self.retarget_enemy(target) {
                    lines.extend(self.skill_hit_enemy(pi, ti, &skill));
                }
            }
        }
        let caster = self.members[pi].name.clone();
        Some(if lines.is_empty() {
            format!("{caster} varázsol")
        } else {
            lines.join("\n")
        })
    }

    /// The screen anchors party member `pi`'s cast of `skill` at `target`
    /// animates over: every living foe for an all-enemy skill (scope 1), the
    /// caster's own party slot for a self-heal (scope 2), the chosen ally's slot
    /// for a single-ally heal (scope 3, empty if that ally is down), every living
    /// ally for an all-ally heal (scope 4), or the single targeted foe otherwise
    /// (falling back to any living foe if that one has fallen). Draws no RNG, so it
    /// can be computed up front to queue the animation before the hold.
    fn skill_anim_anchors(&self, pi: usize, skill: &SkillDef, target: usize) -> Vec<(f32, f32)> {
        match skill.scope {
            1 => self
                .living_enemies()
                .iter()
                .map(|&ti| self.foe_anim_pos(ti))
                .collect(),
            2 => vec![(self.party_anim_x(pi), PARTY_ANIM_Y)],
            3 => {
                if self.members.get(target).is_some_and(|m| m.alive()) {
                    vec![(self.party_anim_x(target), PARTY_ANIM_Y)]
                } else {
                    Vec::new()
                }
            }
            4 => self
                .living_members()
                .iter()
                .map(|&ti| (self.party_anim_x(ti), PARTY_ANIM_Y))
                .collect(),
            _ => {
                let ti = if self.enemies.get(target).is_some_and(|e| e.alive()) {
                    Some(target)
                } else {
                    self.living_enemies().first().copied()
                };
                ti.map(|ti| vec![self.foe_anim_pos(ti)]).unwrap_or_default()
            }
        }
    }

    /// Land `skill` from caster `pi` on enemy `ti`: elemental damage against the
    /// foe's resistance ranks (SP drains find no pool on a foe), an optional
    /// life-absorb for the caster, and a status-infliction roll per affected
    /// state, weighted by the foe's affliction rank. The skill's battle animation
    /// is queued once for the whole cast by [`Battle::cast_skill`], not here.
    fn skill_hit_enemy(&mut self, pi: usize, ti: usize, skill: &SkillDef) -> Vec<String> {
        let caster = self.members[pi].name.clone();
        let target = self.enemies[ti].name.clone();
        // Roll the skill's to-hit (EasyRPG `CalcSkillToHit`): its own hit rate, or
        // the bare-hands 90% default when unset (0), agility-adjusted — but a
        // certain hit against a foe that cannot act. A miss deals nothing and
        // inflicts no state.
        let can_act = logic::worst_restriction(&self.enemies[ti].states, &self.states) != 1;
        let hit = logic::to_hit_vs(
            logic::effective_hit(skill.hit),
            self.members[pi].stats.agility,
            self.enemies[ti].stats.agility,
            can_act,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
            let pos = self.foe_anim_pos(ti);
            self.pending_se.push(BattleSe::Dodge);
            self.push_number(pos, "Miss".to_string(), NumberKind::Miss);
            return vec![format!("{caster} varázsol: {target} elkerülte")];
        }
        let base = logic::skill_effect(
            skill,
            &self.members[pi].stats,
            &self.enemies[ti].stats,
            true,
        );
        let element = skill.attributes.first().copied().unwrap_or(0);
        let base = logic::elemental_damage(
            base,
            element,
            &self.enemies[ti].attribute_ranks,
            &self.attributes,
        );
        // A skill can land a critical (EasyRPG `CalcSkillEffect` triples the
        // effect), rolled off the caster's weapon crit rate like a normal attack.
        let crit = ((rng_next(&mut self.rng) % 100) as u32) < self.members[pi].weapon_crit;
        let base = if crit {
            logic::critical_damage(base)
        } else {
            base
        };
        // Foes carry no SP pool, so an SP-draining skill finds nothing to take.
        let dealt = if skill.affect_sp {
            0
        } else {
            self.hit_enemy(ti, base, skill.variance as i32)
        };
        if skill.absorb && dealt > 0 {
            let f = &mut self.members[pi];
            f.hp = (f.hp + dealt).min(f.max_hp);
            let pos = (self.party_anim_x(pi), PARTY_ANIM_Y);
            self.push_number(pos, dealt.to_string(), NumberKind::Heal);
        }
        let head = if crit {
            format!("{caster} varázsol: Kritikus! {target} -{dealt}")
        } else {
            format!("{caster} varázsol: {target} -{dealt}")
        };
        let mut lines = vec![head];
        for &sid in &skill.affected_states {
            let rank = sid
                .checked_sub(1)
                .and_then(|i| self.enemies[ti].state_ranks.get(i as usize).copied())
                .unwrap_or(2);
            if ((rng_next(&mut self.rng) % 100) as u32) < logic::state_infliction_chance(rank) {
                logic::inflict(&mut self.enemies[ti].states, sid);
                if let Some(state) = self.states.iter().find(|s| s.id == sid) {
                    lines.push(format!("{target} státusz: {}", state.name));
                }
            }
        }
        lines
    }

    /// Heal ally `ti` for caster `pi`'s `skill`: restore SP or HP (clamped to the
    /// maximum), then cure each of the skill's affected states from that ally. The
    /// skill's battle animation is queued once for the whole cast by
    /// [`Battle::cast_skill`], not here.
    fn skill_heal_ally(&mut self, pi: usize, ti: usize, skill: &SkillDef) -> Vec<String> {
        let caster = self.members[pi].name.clone();
        let target = self.members[ti].name.clone();
        let base = logic::skill_effect(
            skill,
            &self.members[pi].stats,
            &self.members[ti].stats,
            false,
        );
        let roll = rng_next(&mut self.rng);
        let amt = logic::variance_adjust(base, skill.variance as i32, roll).max(0);
        let f = &mut self.members[ti];
        if skill.affect_sp {
            f.sp = (f.sp + amt).min(f.max_sp);
        } else {
            f.hp = (f.hp + amt).min(f.max_hp);
        }
        if amt > 0 {
            let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
            self.push_number(pos, amt.to_string(), NumberKind::Heal);
        }
        let mut lines = vec![format!("{caster} varázsol: {target} +{amt}")];
        for &sid in &skill.affected_states {
            if logic::has_state(&self.members[ti].states, sid) {
                logic::cure(&mut self.members[ti].states, sid);
                if let Some(state) = self.states.iter().find(|s| s.id == sid) {
                    lines.push(format!("{target} gyógyul: {}", state.name));
                }
            }
        }
        lines
    }

    /// Apply item `item_id`, used by member `pi`, to ally `target` (falling back to
    /// the user when that member is gone): restore HP and SP by the item's flat
    /// amount plus its percent-of-maximum (each clamped to the maximum) and cure
    /// each of its `cure_states` from the recipient. The item was already consumed
    /// from the inventory when the order was committed, so an unknown id or an item
    /// with no restorative effect still logs a use line.
    fn apply_item(&mut self, pi: usize, item_id: u32, target: usize) -> String {
        let user = self.members[pi].name.clone();
        let Some(item) = self.items.iter().find(|i| i.id == item_id).cloned() else {
            return format!("{user} használ");
        };
        let ti = if self.members.get(target).is_some_and(|m| m.alive()) {
            target
        } else {
            pi
        };
        let (max_hp, max_sp) = (self.members[ti].max_hp, self.members[ti].max_sp);
        let hp_gain = item.recover_hp as i32 + max_hp * item.recover_hp_rate as i32 / 100;
        let sp_gain = item.recover_sp as i32 + max_sp * item.recover_sp_rate as i32 / 100;
        if hp_gain > 0 {
            self.members[ti].hp = (self.members[ti].hp + hp_gain).min(max_hp);
        }
        if sp_gain > 0 {
            self.members[ti].sp = (self.members[ti].sp + sp_gain).min(max_sp);
        }
        let shown = hp_gain.max(sp_gain);
        if shown > 0 {
            let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
            self.push_number(pos, shown.to_string(), NumberKind::Heal);
        }
        let mut cured: Vec<String> = Vec::new();
        for &sid in &item.cure_states {
            if logic::has_state(&self.members[ti].states, sid) {
                logic::cure(&mut self.members[ti].states, sid);
                if let Some(state) = self.states.iter().find(|s| s.id == sid) {
                    cured.push(state.name.clone());
                }
            }
        }
        let gain = match (hp_gain > 0, sp_gain > 0) {
            (true, true) => format!(" (+{hp_gain} HP, +{sp_gain} SP)"),
            (true, false) => format!(" (+{hp_gain} HP)"),
            (false, true) => format!(" (+{sp_gain} SP)"),
            (false, false) => String::new(),
        };
        let recipient = self.members[ti].name.clone();
        let mut lines = vec![format!("{user} használ: {}{gain}", item.name)];
        for state_name in cured {
            lines.push(format!("{recipient} gyógyul: {state_name}"));
        }
        lines.join("\n")
    }

    /// Choose living enemy `i`'s action for the coming round. A status restriction
    /// overrides the AI: can't-act does nothing, berserk forces a plain attack on a
    /// party member, confusion an attack on a random other foe (friendly fire).
    /// Otherwise a random living party target, this foe's and the party's HP
    /// percentages, and the round number feed [`logic::choose_enemy_action`], whose
    /// result [`logic::enemy_command`] maps to a [`Command`] (a basic attack when
    /// nothing is eligible). `None` when the enemy is down or no member is left to
    /// target.
    pub(super) fn enemy_action(&mut self, i: usize, alive: &[bool]) -> Option<Action> {
        if !self.enemies[i].alive() {
            return None;
        }
        let agility = self.enemies[i].stats.agility;
        let restriction = logic::worst_restriction(&self.enemies[i].states, &self.states);
        let kind = match restriction {
            1 => Command::Nothing,
            3 => {
                // Confusion turns the blow on a random other living foe (resolved
                // as friendly fire in `apply`); none left -> nothing.
                let others: Vec<bool> = self
                    .enemies
                    .iter()
                    .enumerate()
                    .map(|(j, e)| j != i && e.alive())
                    .collect();
                match logic::select_target(&others, rng_next(&mut self.rng) as usize) {
                    Some(target) => Command::Attack { target },
                    None => Command::Nothing,
                }
            }
            2 => {
                // Berserk forces a plain attack on a random living party member.
                let target = logic::select_target(alive, rng_next(&mut self.rng) as usize)?;
                Command::Attack { target }
            }
            _ => {
                let target = logic::select_target(alive, rng_next(&mut self.rng) as usize)?;
                let party_hp: i32 = self.members.iter().map(|f| f.hp.max(0)).sum();
                let party_max: i32 = self.members.iter().map(|f| f.max_hp).sum();
                let enemy_hp_pct = logic::hp_percent(self.enemies[i].hp, self.enemies[i].max_hp);
                let party_hp_pct = logic::hp_percent(party_hp, party_max);
                let chosen = logic::choose_enemy_action(
                    &self.enemies[i].actions,
                    enemy_hp_pct,
                    party_hp_pct,
                    logic::AI_PARTY_LEVEL,
                    self.round,
                    rng_next(&mut self.rng),
                );
                logic::enemy_command(chosen.as_ref(), target)
            }
        };
        Some(Action {
            source: Source::Enemy(i),
            kind,
            agility,
        })
    }

    /// Resolve enemy `ei`'s cast of `skill_id` at member `target`, queuing the
    /// skill's battle animation on whichever side it resolves against: an
    /// ally-scope skill (scope 2/3/4) heals the caster itself (a foe keeps no
    /// ally list) and animates on that foe, clamped to its max HP; any other
    /// scope damages the member (halved once if the member's equipment guards one
    /// of the skill's elements, and again while it defends, like a physical hit)
    /// and animates at that member's party slot. `None` for an unknown skill id.
    fn enemy_cast(&mut self, ei: usize, skill_id: u32, target: usize) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id).cloned()?;
        let name = self.enemies[ei].name.clone();
        // Queue the cast's animation once (suppressed on the deferred replay, when
        // `apply` already queued it before the hold); it draws no RNG, so the
        // effect rolls below keep their order whichever path runs this.
        if !self.suppress_anim {
            let anchors = self.enemy_skill_anim_anchors(ei, &skill, target);
            self.push_anim(skill.animation_id, anchors);
        }
        if matches!(skill.scope, 2..=4) {
            let base = logic::skill_effect(
                &skill,
                &self.enemies[ei].stats,
                &self.enemies[ei].stats,
                false,
            );
            let roll = rng_next(&mut self.rng);
            let amt = logic::variance_adjust(base, skill.variance as i32, roll).max(0);
            let e = &mut self.enemies[ei];
            e.hp = (e.hp + amt).min(e.max_hp);
            if amt > 0 {
                let pos = self.foe_anim_pos(ei);
                self.push_number(pos, amt.to_string(), NumberKind::Heal);
            }
            return Some(format!("{name} varázsol: {name} +{amt}"));
        }
        let ti = self.retarget_member(target)?;
        // Roll the skill's to-hit (EasyRPG `CalcSkillToHit`), certain against a
        // member that cannot act. An enemy skill carries no crit stat in our model,
        // so it never critical's (a documented simplification).
        let can_act = logic::worst_restriction(&self.members[ti].states, &self.states) != 1;
        let hit = logic::to_hit_vs(
            logic::effective_hit(skill.hit),
            self.enemies[ei].stats.agility,
            self.members[ti].stats.agility,
            can_act,
        );
        if (rng_next(&mut self.rng) % 100) as i32 >= hit {
            let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
            self.pending_se.push(BattleSe::Dodge);
            self.push_number(pos, "Miss".to_string(), NumberKind::Miss);
            return Some(format!(
                "{name} varázsol: {} elkerülte",
                self.members[ti].name
            ));
        }
        let base = logic::skill_effect(
            &skill,
            &self.enemies[ei].stats,
            &self.members[ti].stats,
            true,
        );
        // Members carry no A–E element ranks, so an enemy skill's element bites
        // only through the target's equipment: one guarded element halves it
        // (min 1). Only skills that actually carry an attribute can be resisted.
        let resisted = base > 0
            && skill
                .attributes
                .iter()
                .any(|a| self.members[ti].resist_attributes.contains(a));
        let base = if resisted { (base / 2).max(1) } else { base };
        let dmg = self.hit_member(ti, base, skill.variance as i32);
        Some(format!("{name} varázsol: {} -{dmg}", self.members[ti].name))
    }

    /// The screen anchors an enemy `ei`'s cast of `skill` at member `target`
    /// animates over: the casting foe itself for an ally-scope skill (scope 2–4,
    /// which a foe turns on itself), else the targeted member's party slot
    /// (falling back to any living member). Draws no RNG.
    fn enemy_skill_anim_anchors(
        &self,
        ei: usize,
        skill: &SkillDef,
        target: usize,
    ) -> Vec<(f32, f32)> {
        if matches!(skill.scope, 2..=4) {
            vec![self.foe_anim_pos(ei)]
        } else {
            let ti = if self.members.get(target).is_some_and(|m| m.alive()) {
                Some(target)
            } else {
                self.living_members().first().copied()
            };
            ti.map(|ti| vec![(self.party_anim_x(ti), PARTY_ANIM_Y)])
                .unwrap_or_default()
        }
    }

    /// Keep `target` if that enemy still lives, else pick another living enemy.
    fn retarget_enemy(&mut self, target: usize) -> Option<usize> {
        if self.enemies.get(target).is_some_and(|e| e.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self.enemies.iter().map(|e| e.alive()).collect();
        let roll = rng_next(&mut self.rng) as usize;
        logic::select_target(&alive, roll)
    }

    fn retarget_member(&mut self, target: usize) -> Option<usize> {
        if self.members.get(target).is_some_and(|m| m.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self.members.iter().map(|m| m.alive()).collect();
        let roll = rng_next(&mut self.rng) as usize;
        logic::select_target(&alive, roll)
    }

    /// Keep confused member `pi`'s stored ally `target` if it still lives, else pick
    /// another living ally. `None` when `pi` has no living ally to turn on.
    fn retarget_ally(&mut self, pi: usize, target: usize) -> Option<usize> {
        if target != pi && self.members.get(target).is_some_and(|m| m.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self
            .members
            .iter()
            .enumerate()
            .map(|(j, m)| j != pi && m.alive())
            .collect();
        logic::select_target(&alive, rng_next(&mut self.rng) as usize)
    }

    /// Keep confused foe `ei`'s stored fellow `target` if it still lives, else pick
    /// another living foe. `None` when `ei` has no living fellow to turn on.
    fn retarget_other_enemy(&mut self, ei: usize, target: usize) -> Option<usize> {
        if target != ei && self.enemies.get(target).is_some_and(|e| e.alive()) {
            return Some(target);
        }
        let alive: Vec<bool> = self
            .enemies
            .iter()
            .enumerate()
            .map(|(j, e)| j != ei && e.alive())
            .collect();
        logic::select_target(&alive, rng_next(&mut self.rng) as usize)
    }

    /// Advance the command phase past every member the game must act for: a
    /// can't-act (restriction 1) member is auto-ordered [`Command::Nothing`]; a
    /// berserk (2) member is forced to strike a random living enemy; a confused (3)
    /// member a random living ally. Stops on the first member who may freely choose
    /// (setting [`Battle::turn`]), or enters resolution once every remaining chooser
    /// has been auto-ordered — so the command UI never halts on a restricted member.
    pub(super) fn skip_restricted_choosers(&mut self) {
        while let Some(i) = self.next_chooser() {
            let restriction = logic::worst_restriction(&self.members[i].states, &self.states);
            if restriction == 0 {
                self.turn = i;
                return;
            }
            let forced = self.forced_party_command(i, restriction);
            self.members[i].command = Some(forced);
        }
        self.begin_resolve();
    }

    /// Order every living member for RM2000 Auto-battle, then resolve: a freely
    /// acting member basic-attacks a random living enemy (nothing when no foe is
    /// left), while a restricted member keeps its forced action. Reuses the same
    /// target/AI helpers as the per-actor flow, so the enemies still act.
    pub(super) fn auto_battle_commands(&mut self) {
        let enemies_alive: Vec<bool> = self.enemies.iter().map(|e| e.alive()).collect();
        for i in 0..self.members.len() {
            if !self.members[i].alive() {
                continue;
            }
            let restriction = logic::worst_restriction(&self.members[i].states, &self.states);
            let command = if restriction == 0 {
                match logic::select_target(&enemies_alive, rng_next(&mut self.rng) as usize) {
                    Some(target) => Command::Attack { target },
                    None => Command::Nothing,
                }
            } else {
                self.forced_party_command(i, restriction)
            };
            self.members[i].command = Some(command);
        }
        self.begin_resolve();
    }

    /// The attack a restricted member `i` is forced into: a random living enemy
    /// while berserk (2), a random living ally while confused (3), or nothing at all
    /// (restriction 1, or no legal target). The chosen side is re-checked and the
    /// blow re-aimed at resolution by [`Battle::apply`].
    fn forced_party_command(&mut self, i: usize, restriction: u32) -> Command {
        let alive: Vec<bool> = match restriction {
            2 => self.enemies.iter().map(|e| e.alive()).collect(),
            3 => self
                .members
                .iter()
                .enumerate()
                .map(|(j, m)| j != i && m.alive())
                .collect(),
            _ => return Command::Nothing,
        };
        match logic::select_target(&alive, rng_next(&mut self.rng) as usize) {
            Some(target) => Command::Attack { target },
            None => Command::Nothing,
        }
    }

    /// Wear off timed states at the top of a round: for every combatant advance
    /// each active state's held-turn count and roll its auto-release once past its
    /// hold turns (the death state is exempt), logging whatever lifts. Per-turn HP
    /// change (poison drain, regen) is applied separately, at the start of each
    /// battler's turn, by [`Battle::tick_state_hp`].
    pub(super) fn run_recovery(&mut self) {
        let Battle {
            members,
            enemies,
            states,
            rng,
            log,
            ..
        } = self;
        let defs = states.as_slice();
        for f in members.iter_mut() {
            for id in logic::tick_recovery(&mut f.states, defs, || (rng_next(rng) % 100) as u32) {
                if let Some(d) = defs.iter().find(|d| d.id == id) {
                    log.push(format!("{}: {} elmúlt", f.name, d.name));
                }
            }
        }
        for e in enemies.iter_mut() {
            for id in logic::tick_recovery(&mut e.states, defs, || (rng_next(rng) % 100) as u32) {
                if let Some(d) = defs.iter().find(|d| d.id == id) {
                    log.push(format!("{}: {} elmúlt", e.name, d.name));
                }
            }
        }
    }

    /// Roll the damage wear-off of member `ti`'s active states after it is struck,
    /// logging whatever lifts.
    fn release_states_on_member(&mut self, ti: usize) {
        let Battle {
            members,
            states,
            rng,
            log,
            ..
        } = self;
        let defs = states.as_slice();
        let lifted = logic::release_on_damage(&mut members[ti].states, defs, || {
            (rng_next(rng) % 100) as u32
        });
        for id in lifted {
            if let Some(d) = defs.iter().find(|d| d.id == id) {
                log.push(format!("{}: {} elmúlt", members[ti].name, d.name));
            }
        }
    }

    /// Roll the damage wear-off of foe `ti`'s active states after it is struck,
    /// logging whatever lifts.
    fn release_states_on_enemy(&mut self, ti: usize) {
        let Battle {
            enemies,
            states,
            rng,
            log,
            ..
        } = self;
        let defs = states.as_slice();
        let lifted = logic::release_on_damage(&mut enemies[ti].states, defs, || {
            (rng_next(rng) % 100) as u32
        });
        for id in lifted {
            if let Some(d) = defs.iter().find(|d| d.id == id) {
                log.push(format!("{}: {} elmúlt", enemies[ti].name, d.name));
            }
        }
    }

    /// The terminal outcome, if reached: victory when every enemy is down, defeat
    /// when every member is. `None` while the fight continues.
    pub fn end_state(&self) -> Option<BattleOutcome> {
        if self.enemies.iter().all(|e| !e.alive()) {
            Some(BattleOutcome::Victory)
        } else if self.members.iter().all(|m| !m.alive()) {
            Some(BattleOutcome::Defeat)
        } else {
            None
        }
    }

    /// Total `(exp, gold)` for defeating the troop: every foe actually beaten. A
    /// fled foe (RM2000 Escape) is not defeated, so it grants nothing.
    pub fn victory_rewards(&self) -> (u32, u32) {
        let rewards: Vec<(u32, u32)> = self
            .enemies
            .iter()
            .filter(|e| !e.fled)
            .map(|e| (e.exp, e.gold))
            .collect();
        logic::total_rewards(&rewards)
    }

    /// Roll a party escape against the persisted RM2000 escape chance (EasyRPG
    /// `TryEscape`): a first strike escapes outright; otherwise a `PercentChance`
    /// draw against `escape_chance`. On failure the chance is raised by 10 for the
    /// next attempt and `false` is returned, so the caller forfeits the party's
    /// turn. The chance was fixed at [`Battle::build`] from the two sides' average
    /// agilities and is never recomputed here.
    pub fn attempt_escape(&mut self) -> bool {
        if self.first_strike {
            return true;
        }
        let roll = (rng_next(&mut self.rng) % 100) as u32;
        if logic::escape_succeeds(self.escape_chance, roll) {
            true
        } else {
            self.escape_chance += 10;
            false
        }
    }

    /// Conclude the fight with `outcome` and wait in the outcome phase for the
    /// player. On victory this records the reward and stages the RM2000
    /// `ProcessSceneActionVictory` message run as separate log lines — the victory
    /// term, then the experience and gold gained. The reward is *paid* (gold added,
    /// experience awarded, level-ups appended) by `battle::apply_victory_rewards`
    /// once the outcome is entered, since that needs the party's progression and
    /// inventory resources.
    pub fn finish(&mut self, outcome: BattleOutcome) {
        self.outcome = Some(outcome);
        self.phase = Phase::Outcome;
        match outcome {
            BattleOutcome::Victory => {
                let (exp, gold) = self.victory_rewards();
                self.reward_exp = exp;
                self.reward_gold = gold;
                self.log.push(i18n::tr(&self.text.victory));
                if exp > 0 {
                    // RM2000 (2000) message order: "<value><exp_received>".
                    self.log
                        .push(format!("{exp}{}", i18n::tr(&self.text.exp_received)));
                }
                if gold > 0 {
                    // RM2000 (2000): "<received_a> <value><gold><received_b>".
                    self.log.push(format!(
                        "{} {gold}{}{}",
                        i18n::tr(&self.text.gold_received_a),
                        i18n::tr(&self.text.gold),
                        i18n::tr(&self.text.gold_received_b),
                    ));
                }
            }
            BattleOutcome::Escape => {
                let line = i18n::tr(&self.text.escape_success);
                self.log.push(line);
            }
            BattleOutcome::Defeat => {
                let line = i18n::tr(&self.text.defeat);
                self.log.push(line);
            }
        }
    }
}

#[cfg(test)]
mod tests;
