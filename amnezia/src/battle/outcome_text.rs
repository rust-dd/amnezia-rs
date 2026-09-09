use super::model::Battle;

const PAGE_ROWS: usize = 3;

fn lines(battle: &Battle) -> Vec<String> {
    let start = battle.outcome_log_start.min(battle.log.len());
    wrap(&battle.log[start..].join("\n"), 50)
}

pub(super) fn page(battle: &Battle) -> Vec<String> {
    lines(battle)
        .into_iter()
        .skip(battle.outcome_page * PAGE_ROWS)
        .take(PAGE_ROWS)
        .collect()
}

pub(super) fn advance(battle: &mut Battle) -> bool {
    if (battle.outcome_page + 1) * PAGE_ROWS < lines(battle).len() {
        battle.outcome_page += 1;
        true
    } else {
        false
    }
}

pub(super) fn wrap(text: &str, cells: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut current = String::new();
        for word in line.split_whitespace() {
            if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > cells {
                lines.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::BattleOutcome;
    use crate::battle::model::testkit::build_1v2;

    #[test]
    fn every_reward_and_level_up_line_remains_available_before_leaving_battle() {
        let mut battle = build_1v2();
        battle.finish(BattleOutcome::Victory);
        battle.log.extend([
            "Jégkarom megszerezve!".into(),
            "Ron szintet lépett!".into(),
            "Tiffany szintet lépett!".into(),
            "Daren szintet lépett!".into(),
        ]);
        let expected = battle.log[battle.outcome_log_start..].to_vec();
        let mut shown = Vec::new();
        loop {
            let page = page(&battle);
            assert!(page.len() <= 3);
            shown.extend(page);
            if !advance(&mut battle) {
                break;
            }
        }
        assert_eq!(shown, expected);
        assert!(!advance(&mut battle));
    }
}
