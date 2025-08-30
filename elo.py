# completely chatgpt generated, take with a grain of salt
import math
from scipy.stats import norm

def elo_diff_from_results(wins, draws, losses, confidence=0.95):
    n = wins + draws + losses
    if n == 0:
        raise ValueError("No games played.")

    # observed score (win=1, draw=0.5, loss=0)
    score = (wins + 0.5 * draws) / n

    # avoid edge cases of exactly 0 or 1
    eps = 1e-6
    score = min(max(score, eps), 1 - eps)

    # expected score as function of Elo diff:
    # score = 1 / (1 + 10^(-elo/400))
    elo_diff = -400 * math.log10(1/score - 1)

    # standard error of proportion
    se = math.sqrt(score * (1 - score) / n)

    # z-score for CI
    z = norm.ppf(0.5 + confidence/2)

    # CI for score
    lo = max(eps, score - z * se)
    hi = min(1 - eps, score + z * se)

    # convert CI bounds to Elo
    elo_lo = -400 * math.log10(1/lo - 1)
    elo_hi = -400 * math.log10(1/hi - 1)

    return elo_diff, (elo_lo, elo_hi)

# Example usage:
if __name__ == "__main__":
    wins = int(input("Enter number of wins: "))
    draws = int(input("Enter number of draws: "))
    losses = int(input("Enter number of losses: "))
    elo, ci = elo_diff_from_results(wins, draws, losses)
    print(f"Elo difference estimate: {elo:.1f}")
    print(f"95% confidence interval: [{ci[0]:.1f}, {ci[1]:.1f}]")
