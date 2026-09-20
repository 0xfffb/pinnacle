//! Heuristic UA / rate / path detector.

use pinnacle_core::{Action, Context};

use super::{Detector, RiskVerdict};

#[derive(Debug, Default, Clone, Copy)]
pub struct HeuristicDetector;

impl HeuristicDetector {
    fn is_bot_like_ua(ua: &str) -> bool {
        let lower = ua.to_ascii_lowercase();
        [
            "bot",
            "spider",
            "crawler",
            "scrapy",
            "curl",
            "python-requests",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
    }

    fn score_action(score: u8) -> Action {
        // Passed traffic only: allow clean, re-challenge suspicious, block obvious bots.
        match score {
            0..=29 => Action::Allow,
            30..=69 => Action::Challenge,
            _ => Action::Block,
        }
    }
}

impl Detector for HeuristicDetector {
    fn evaluate(&self, ctx: &Context) -> RiskVerdict {
        let mut score: u16 = 0;
        let mut reasons = Vec::new();

        let ua = ctx.get_or(pinnacle_core::USER_AGENT, "");
        let request_count = ctx.get_u32(pinnacle_core::REQUEST_COUNT);
        let path = ctx.get_or(pinnacle_core::PATH, "");

        if ua.trim().is_empty() {
            score += 40;
            reasons.push("missing_user_agent");
        } else if Self::is_bot_like_ua(ua) {
            score += 35;
            reasons.push("bot_like_user_agent");
        }

        if request_count > 100 {
            score += 40;
            reasons.push("high_request_rate");
        } else if request_count > 30 {
            score += 20;
            reasons.push("elevated_request_rate");
        }

        if path.contains("..") {
            score += 25;
            reasons.push("suspicious_path");
        }

        let score = score.min(100) as u8;
        RiskVerdict {
            score,
            action: Self::score_action(score),
            reasons,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinnacle_core::Action;

    #[test]
    fn allows_normal_browser_traffic() {
        let verdict =
            HeuristicDetector.evaluate(&Context::new("/api/items", "203.0.113.10", "Mozilla/5.0"));
        assert_eq!(verdict.action, Action::Allow);
        assert!(verdict.score < 30);
    }

    #[test]
    fn challenges_bot_like_user_agent() {
        let verdict = HeuristicDetector.evaluate(&Context::new(
            "/api/items",
            "203.0.113.10",
            "python-requests/2.31",
        ));
        assert_eq!(verdict.action, Action::Challenge);
    }

    #[test]
    fn blocks_high_rate_scrapers() {
        let verdict = HeuristicDetector
            .evaluate(&Context::new("/api/items", "203.0.113.10", "scrapy/2.11").with_count(200));
        assert_eq!(verdict.action, Action::Block);
        assert!(verdict.score >= 70);
    }
}
