use blok_rs::board::{BoardState, GameResult, Player, StartPosition};
use blok_rs::movegen::generate_moves;
use rand::rng;
use rand::seq::IndexedRandom;
use rayon::prelude::*;
use std::process::{Command, Stdio};

/// Path to the two engine executables to compare.
const ENGINE1_PATH: &str = "./executables/ab-test";
const ENGINE2_PATH: &str = "./executables/ab-latest";

const DEFAULT_MOVETIME_MS: usize = 1000;
const DEFAULT_BASELINE_PAIRS: usize = 80;

const DEFAULT_OPENING_PLIES: usize = 6;
const PARALLEL_PAIRS: usize = 4;

const ALPHA: f64 = 0.05;
const BETA: f64 = 0.05;

/// Regularization for empty pentanomial bins (Fishtest `regularize`).
const PRIOR: f64 = 1e-3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Win,
    Draw,
    Lose,
}

impl Outcome {
    fn score(self) -> f64 {
        match self {
            Outcome::Win => 1.0,
            Outcome::Draw => 0.5,
            Outcome::Lose => 0.0,
        }
    }
}

#[derive(PartialEq, Eq)]
enum SPRTResult {
    SignificantNull,
    SignificantAlt,
    NotSignificant,
}

/// Pair score in half-points: LL=0, LD=1, DDL/WL=2, WD=3, WW=4.
#[derive(Clone, Copy)]
struct PairOutcome {
    half_points: usize,
    /// True when the 1-1 pair was WL (same color won both), not DD.
    wl: bool,
}

fn pair_from_games(first: Outcome, second: Outcome) -> PairOutcome {
    let half_points = (2.0 * (first.score() + second.score())).round() as usize;
    let wl = matches!(
        (first, second),
        (Outcome::Win, Outcome::Lose) | (Outcome::Lose, Outcome::Win)
    );
    PairOutcome { half_points, wl }
}

/// Fishtest pentanomial GSPRT (`LLR_logistic`): 5-category multinomial
/// over color-swapped game pairs, logistic Elo on the per-game score.
struct PentanomialSPRT {
    /// [LL, LD, DD/WL, WD, WW]
    counts: [f64; 5],
    wl: usize,
    dd: usize,
    elo_0: f64,
    elo_1: f64,
}

impl PentanomialSPRT {
    fn new(elo_0: f64, elo_1: f64) -> Self {
        Self {
            counts: [0.0; 5],
            wl: 0,
            dd: 0,
            elo_0,
            elo_1,
        }
    }

    fn update(&mut self, pair: PairOutcome) {
        self.counts[pair.half_points] += 1.0;
        if pair.half_points == 2 {
            if pair.wl {
                self.wl += 1;
            } else {
                self.dd += 1;
            }
        }
    }

    fn n_pairs(&self) -> usize {
        self.counts.iter().sum::<f64>() as usize
    }

    fn wdl(&self) -> (usize, usize, usize) {
        let ww = self.counts[4];
        let wd = self.counts[3];
        let ld = self.counts[1];
        let ll = self.counts[0];
        let wins = (2.0 * ww + wd + self.wl as f64).round() as usize;
        let losses = (2.0 * ll + ld + self.wl as f64).round() as usize;
        let draws = (wd + ld + 2.0 * self.dd as f64).round() as usize;
        (wins, draws, losses)
    }

    fn score(&self) -> f64 {
        let n = self.n_pairs();
        if n == 0 {
            return 0.5;
        }
        let points: f64 = self
            .counts
            .iter()
            .enumerate()
            .map(|(i, c)| *c * (i as f64) / 4.0)
            .sum();
        points / n as f64
    }

    fn llr(&self) -> f64 {
        llr_logistic(self.elo_0, self.elo_1, &self.counts)
    }

    fn bounds() -> (f64, f64) {
        ((BETA / (1.0 - ALPHA)).ln(), ((1.0 - BETA) / ALPHA).ln())
    }

    fn result(&self) -> SPRTResult {
        if self.n_pairs() == 0 {
            return SPRTResult::NotSignificant;
        }
        let (a, b) = Self::bounds();
        let llr = self.llr();
        if llr < a {
            SPRTResult::SignificantNull
        } else if llr > b {
            SPRTResult::SignificantAlt
        } else {
            SPRTResult::NotSignificant
        }
    }

    fn print_line(&self) {
        let (w, d, l) = self.wdl();
        let (a, b) = Self::bounds();
        println!(
            "Ptnml [{:.0}, {:.0}, {:.0}, {:.0}, {:.0}] wl={} dd={}  WDL {} {} {}  score {:.1}%  LLR {:.2} ({:.2}, {:.2})",
            self.counts[0],
            self.counts[1],
            self.counts[2],
            self.counts[3],
            self.counts[4],
            self.wl,
            self.dd,
            w,
            d,
            l,
            100.0 * self.score(),
            self.llr(),
            a,
            b
        );
    }
}

fn elo_to_score(elo: f64) -> f64 {
    1.0 / (1.0 + 10.0_f64.powf(-elo / 400.0))
}

fn results_to_pdf(results: &[f64; 5]) -> (f64, [(f64, f64); 5]) {
    let mut regularized = *results;
    for p in &mut regularized {
        if *p == 0.0 {
            *p = PRIOR;
        }
    }
    let n: f64 = regularized.iter().sum();
    let mut pdf = [(0.0, 0.0); 5];
    for i in 0..5 {
        pdf[i] = (i as f64 / 4.0, regularized[i] / n);
    }
    (n, pdf)
}

/// Solve Σ p_i a_i / (1 + x a_i) = 0. Support of `a` must straddle 0.
fn secular(pdf: &[(f64, f64); 5]) -> f64 {
    let mut v = f64::INFINITY;
    let mut w = f64::NEG_INFINITY;
    for &(ai, _) in pdf {
        v = v.min(ai);
        w = w.max(ai);
    }
    assert!(
        v * w < 0.0,
        "secular equation needs support straddling 0 (v={v}, w={w})"
    );
    let eps = 1e-9;
    let mut lo = -1.0 / w + eps;
    let mut hi = -1.0 / v - eps;

    let f = |x: f64| {
        pdf.iter()
            .map(|&(ai, pi)| pi * ai / (1.0 + x * ai))
            .sum::<f64>()
    };

    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if f(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

fn mle_expected(pdf: &[(f64, f64); 5], s: f64) -> [(f64, f64); 5] {
    let shifted = [
        (pdf[0].0 - s, pdf[0].1),
        (pdf[1].0 - s, pdf[1].1),
        (pdf[2].0 - s, pdf[2].1),
        (pdf[3].0 - s, pdf[3].1),
        (pdf[4].0 - s, pdf[4].1),
    ];
    let x = secular(&shifted);
    let mut out = [(0.0, 0.0); 5];
    for i in 0..5 {
        let (ai, pi) = pdf[i];
        out[i] = (ai, pi / (1.0 + x * (ai - s)));
    }
    out
}

fn llr_logistic(elo0: f64, elo1: f64, results: &[f64; 5]) -> f64 {
    let s0 = elo_to_score(elo0);
    let s1 = elo_to_score(elo1);
    let (n, pdf) = results_to_pdf(results);
    let pdf0 = mle_expected(&pdf, s0);
    let pdf1 = mle_expected(&pdf, s1);
    let mut llr = 0.0;
    for i in 0..5 {
        llr += pdf[i].1 * (pdf1[i].1.ln() - pdf0[i].1.ln());
    }
    n * llr
}

fn outcome_for(result: GameResult, engine1_is_white: bool) -> Outcome {
    match (result, engine1_is_white) {
        (GameResult::Win(Player::White), true) | (GameResult::Win(Player::Black), false) => {
            Outcome::Win
        }
        (GameResult::Win(Player::White), false) | (GameResult::Win(Player::Black), true) => {
            Outcome::Lose
        }
        (GameResult::Draw, _) => Outcome::Draw,
        (GameResult::InProgress, _) => unreachable!(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.len() > 5 {
        eprintln!(
            "Usage: {} <gain|nonregr|baseline> [movetime_ms] [baseline_pairs] [opening_plies]",
            args[0]
        );
        std::process::exit(1);
    }

    let mode = args[1].as_str();
    let (elo_0, elo_1, baseline) = match mode {
        "gain" => (0.0, 5.0, false),
        "nonregr" => (-10.0, 0.0, false),
        "baseline" => (0.0, 5.0, true),
        _ => {
            eprintln!("Invalid mode: {mode}. Use 'gain', 'nonregr', or 'baseline'");
            std::process::exit(1);
        }
    };

    let movetime_ms: usize = if args.len() >= 3 {
        args[2].parse().unwrap_or_else(|_| {
            eprintln!("Invalid movetime_ms: {}", args[2]);
            std::process::exit(1);
        })
    } else if baseline {
        100
    } else {
        DEFAULT_MOVETIME_MS
    };

    let baseline_pairs: usize = if baseline && args.len() >= 4 {
        args[3].parse().unwrap_or_else(|_| {
            eprintln!("Invalid baseline_pairs: {}", args[3]);
            std::process::exit(1);
        })
    } else {
        DEFAULT_BASELINE_PAIRS
    };

    let opening_plies: usize = if args.len() >= 5 {
        args[4].parse().unwrap_or_else(|_| {
            eprintln!("Invalid opening_plies: {}", args[4]);
            std::process::exit(1);
        })
    } else {
        DEFAULT_OPENING_PLIES
    };

    let engine1 = if baseline { ENGINE2_PATH } else { ENGINE1_PATH };
    let engine2 = ENGINE2_PATH;

    if baseline {
        println!(
            "Self-play baseline: {engine2} vs itself, {baseline_pairs} pairs, oneshot TC, {opening_plies} opening plies (movetime arg={movetime_ms} unused)"
        );
        println!("Ptnml bins: LL, LD, DD/WL, WD, WW. wl = same color won both games.");
    } else {
        println!("Starting comparison between {engine1} and {engine2}");
        println!("oneshot engines (compiled TC); movetime arg={movetime_ms} is unused");
        println!("SPRT elo bounds: {elo_0} - {elo_1} (pentanomial GSPRT, logistic Elo)");
    }

    let mut sprt = PentanomialSPRT::new(elo_0, elo_1);

    while {
        if baseline {
            sprt.n_pairs() < baseline_pairs
        } else {
            sprt.result() == SPRTResult::NotSignificant
        }
    } {
        let openings: Vec<Vec<u32>> = (0..PARALLEL_PAIRS)
            .map(|_| generate_opening(opening_plies))
            .collect();
        let pairs: Vec<PairOutcome> = openings
            .par_iter()
            .map(|opening| {
                let (g1, g2) = rayon::join(
                    || play_game(engine1, engine2, opening),
                    || play_game(engine2, engine1, opening),
                );
                pair_from_games(outcome_for(g1, true), outcome_for(g2, false))
            })
            .collect();

        for pair in pairs {
            if baseline && sprt.n_pairs() >= baseline_pairs {
                break;
            }
            sprt.update(pair);
        }
        sprt.print_line();
    }

    if baseline {
        print_baseline_summary(&sprt);
    } else {
        match sprt.result() {
            SPRTResult::SignificantNull => println!("Significant Null"),
            SPRTResult::SignificantAlt => println!("Significant Alt"),
            SPRTResult::NotSignificant => println!("Not Significant"),
        }
    }
}

fn print_baseline_summary(sprt: &PentanomialSPRT) {
    let n = sprt.n_pairs() as f64;
    let ww_ll = sprt.counts[0] + sprt.counts[4];
    let wl_frac = sprt.wl as f64 / n;
    let decisive_frac = ww_ll / n;
    println!();
    println!("Baseline summary ({} pairs):", sprt.n_pairs());
    println!(
        "  2-0 or 0-2: {:.0} ({:.1}%)  — same engine won both colors (think-time noise)",
        ww_ll,
        100.0 * decisive_frac
    );
    println!(
        "  1-1 from WL: {} ({:.1}%)  — same color won both games (opening / first-player bias)",
        sprt.wl,
        100.0 * wl_frac
    );
    println!(
        "  1-1 from DD: {} ({:.1}%)",
        sprt.dd,
        100.0 * sprt.dd as f64 / n
    );
    if wl_frac > 0.4 {
        println!(
            "  High WL rate: 6-ply random openings often decide the pair (color / first-player lock)."
        );
    } else if decisive_frac > 0.25 {
        println!(
            "  High 2-0/0-2 in self-play: time-control noise is large; longer TC or more pairs."
        );
    } else {
        println!("  Openings look usable: most pairs are not color-locked.");
    }
}

fn play_game(engine_white: &str, engine_black: &str, opening_moves: &[u32]) -> GameResult {
    let mut board = BoardState::new(StartPosition::Corner);
    let mut move_strings: Vec<String> = Vec::new();

    for &m in opening_moves {
        board.do_move(m);
        move_strings.push(m.to_string());
    }

    let mut current_engine = if opening_moves.len() % 2 == 0 {
        engine_white
    } else {
        engine_black
    };

    while board.game_result() == GameResult::InProgress {
        let moves_arg = move_strings.join(" ");
        let output = Command::new(current_engine)
            .arg(&moves_arg)
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to run engine executable");

        if !output.status.success() {
            eprintln!(
                "Engine {current_engine} failed to run or returned error. Moves: {moves_arg}"
            );
            std::process::exit(1);
        }

        let best_move_str = String::from_utf8_lossy(&output.stdout);
        let best_move_str = best_move_str.trim();

        let legal_moves = generate_moves(&board);
        let parsed_move = legal_moves
            .iter()
            .find(|&&m| m.to_string() == best_move_str);

        let chosen_move = match parsed_move {
            Some(&m) => m,
            None => {
                eprintln!(
                    "Engine {current_engine} returned illegal or unrecognized move: '{best_move_str}'. Legal moves: {:?}",
                    legal_moves
                        .iter()
                        .map(|m| m.to_string())
                        .collect::<Vec<_>>()
                );
                std::process::exit(1);
            }
        };

        board.do_move(chosen_move);
        move_strings.push(chosen_move.to_string());
        current_engine = if current_engine == engine_white {
            engine_black
        } else {
            engine_white
        };
    }

    board.game_result()
}

fn generate_opening(plies: usize) -> Vec<u32> {
    let mut board = BoardState::new(StartPosition::Corner);
    let mut moves: Vec<u32> = Vec::new();
    let mut rng = rng();

    for _ in 0..plies {
        let legal_moves = generate_moves(&board);
        if legal_moves.is_empty() {
            break;
        }
        let &chosen_move = legal_moves.choose(&mut rng).unwrap();
        board.do_move(chosen_move);
        moves.push(chosen_move);
    }
    moves
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_bins() {
        assert_eq!(pair_from_games(Outcome::Lose, Outcome::Lose).half_points, 0);
        assert_eq!(pair_from_games(Outcome::Win, Outcome::Win).half_points, 4);
        let wl = pair_from_games(Outcome::Win, Outcome::Lose);
        assert_eq!(wl.half_points, 2);
        assert!(wl.wl);
        let dd = pair_from_games(Outcome::Draw, Outcome::Draw);
        assert_eq!(dd.half_points, 2);
        assert!(!dd.wl);
    }

    #[test]
    fn equal_sample_llr_near_zero() {
        // Uniform pentanomial has mean 0.5, matching 0 Elo. H1=5 Elo is worse.
        let llr = llr_logistic(0.0, 5.0, &[20.0, 20.0, 20.0, 20.0, 20.0]);
        assert!(llr < 0.0, "llr={llr}");
        assert!(llr > -1.0, "llr={llr}");
    }

    #[test]
    fn winning_sample_positive_llr() {
        let all_ww = llr_logistic(0.0, 5.0, &[0.0, 0.0, 0.0, 0.0, 40.0]);
        assert!(all_ww > 0.0, "all_ww llr={all_ww}");
        let mild = llr_logistic(0.0, 5.0, &[10.0, 15.0, 30.0, 25.0, 20.0]);
        assert!((mild - 0.51).abs() < 0.05, "mild llr={mild}");
    }
}
