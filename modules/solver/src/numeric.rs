use crate::jacobian::{finite_difference_jacobian_generic, Jacobian};
use crate::residual::{evaluate_residuals_generic, ConstraintResidual, ResidualEquation};
use crate::variables::VarSet;

/// Default maximum number of Gauss-Newton iterations.
pub const DEFAULT_MAX_ITERATIONS: usize = 50;
/// Default residual convergence tolerance (in SI units for dimensional terms).
pub const DEFAULT_RESIDUAL_TOLERANCE: f64 = 1e-9;
/// Default damping seed for the normal-equation solve.
pub const DEFAULT_DAMPING: f64 = 1e-4;
/// Default factor used when reducing/increasing the damping value.
pub const DEFAULT_DAMPING_GROWTH: f64 = 10.0;
/// Default upper bound for the adaptive damping value.
pub const DEFAULT_MAX_DAMPING: f64 = 1e6;
/// Pivot tolerance used when factoring the damped normal equations.
pub const NORMAL_EQUATION_PIVOT_TOLERANCE: f64 = 1e-14;

/// Solver configuration.
#[derive(Debug, Clone)]
pub struct SolverOptions {
    pub max_iterations: usize,
    pub tolerance: f64,
    pub damping: f64,
    pub damping_growth: f64,
    pub max_damping: f64,
}

impl Default for SolverOptions {
    fn default() -> Self {
        Self {
            max_iterations: DEFAULT_MAX_ITERATIONS,
            tolerance: DEFAULT_RESIDUAL_TOLERANCE,
            damping: DEFAULT_DAMPING,
            damping_growth: DEFAULT_DAMPING_GROWTH,
            max_damping: DEFAULT_MAX_DAMPING,
        }
    }
}

/// Result of a numeric solve attempt.
#[derive(Debug, Clone, PartialEq)]
pub struct SolveOutput {
    pub vars: VarSet,
    pub iterations: usize,
    pub max_error: f64,
    pub converged: bool,
}

/// Gauss-Newton with optional Levenberg-Marquardt-style damping.
pub fn gauss_newton_solve(
    equations: &[ConstraintResidual],
    vars: VarSet,
    options: &SolverOptions,
) -> SolveOutput {
    gauss_newton_solve_generic(equations, vars, options)
}

/// Gauss-Newton solve for any equation type implementing [`ResidualEquation`].
pub fn gauss_newton_solve_generic<E: ResidualEquation>(
    equations: &[E],
    mut vars: VarSet,
    options: &SolverOptions,
) -> SolveOutput {
    let mut lambda = options.damping;
    let mut iterations = 0_usize;
    let mut max_error = residual_max_error(&evaluate_residuals_generic(equations, &vars));

    // Check the initial values even when max_iterations is zero.  This keeps
    // the output self-consistent and makes a zero-iteration solve useful for
    // callers that only want to validate an already-solved state.
    if max_error <= options.tolerance {
        return SolveOutput {
            vars,
            iterations,
            max_error,
            converged: true,
        };
    }

    while iterations < options.max_iterations {
        let residuals = evaluate_residuals_generic(equations, &vars);
        max_error = residual_max_error(&residuals);

        if max_error <= options.tolerance {
            return SolveOutput {
                vars,
                iterations,
                max_error,
                converged: true,
            };
        }

        let jacobian = finite_difference_jacobian_generic(equations, &vars);
        let step = match damped_normal_equations_step(&jacobian, &residuals, lambda) {
            Some(step) => step,
            None => break,
        };

        let mut trial = vars.clone();
        for (i, delta) in step.iter().enumerate() {
            trial.set(crate::variables::VarId(i as u32), vars.values()[i] - delta);
        }

        let trial_residuals = evaluate_residuals_generic(equations, &trial);
        let trial_error = residual_max_error(&trial_residuals);

        if trial_error < max_error {
            vars = trial;
            lambda = (lambda / options.damping_growth).max(options.damping);
            iterations += 1;
            max_error = trial_error;
            if max_error <= options.tolerance {
                return SolveOutput {
                    vars,
                    iterations,
                    max_error,
                    converged: true,
                };
            }
        } else {
            lambda = (lambda * options.damping_growth).min(options.max_damping);
            iterations += 1;
        }
    }

    // The loop's `max_error` describes the point at the beginning of the last
    // iteration.  Re-evaluate after the final accepted trial so the reported
    // error and convergence flag always refer to the returned variables.
    max_error = residual_max_error(&evaluate_residuals_generic(equations, &vars));
    SolveOutput {
        vars,
        iterations,
        max_error,
        converged: max_error <= options.tolerance,
    }
}

/// Return the largest finite residual magnitude.
///
/// A non-finite residual is a failed numeric evaluation, not a zero residual.
/// Representing it as infinity keeps the public `SolveOutput` shape stable and
/// prevents NaN from accidentally satisfying the convergence check.
fn residual_max_error(residuals: &[f64]) -> f64 {
    residuals.iter().fold(0.0, |max_error, residual| {
        if residual.is_finite() {
            max_error.max(residual.abs())
        } else {
            f64::INFINITY
        }
    })
}

/// Solve `(J^T J + lambda I) delta = J^T r` for the update `delta`.
fn damped_normal_equations_step(
    jacobian: &Jacobian,
    residuals: &[f64],
    lambda: f64,
) -> Option<Vec<f64>> {
    let n = jacobian.cols;
    if n == 0 {
        return Some(Vec::new());
    }

    let mut jt_j = vec![0.0; n * n];
    let mut jt_r = vec![0.0; n];

    for (row, r) in residuals.iter().enumerate().take(jacobian.rows) {
        let r = *r;
        for i in 0..n {
            let ji = jacobian.get(row, i);
            jt_r[i] += ji * r;
            for j in 0..n {
                jt_j[i * n + j] += ji * jacobian.get(row, j);
            }
        }
    }

    for i in 0..n {
        jt_j[i * n + i] += lambda;
    }

    solve_symmetric_positive_definite(&jt_j, &jt_r, n)
}

/// Cholesky-based solver for small dense systems.
fn solve_symmetric_positive_definite(a: &[f64], b: &[f64], n: usize) -> Option<Vec<f64>> {
    let mut l = vec![0.0; n * n];

    for i in 0..n {
        for j in 0..=i {
            let mut sum = a[i * n + j];
            for k in 0..j {
                sum -= l[i * n + k] * l[j * n + k];
            }
            if i == j {
                if sum <= NORMAL_EQUATION_PIVOT_TOLERANCE {
                    return None;
                }
                l[i * n + j] = sum.sqrt();
            } else {
                l[i * n + j] = sum / l[j * n + j];
            }
        }
    }

    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut sum = b[i];
        for k in 0..i {
            sum -= l[i * n + k] * y[k];
        }
        y[i] = sum / l[i * n + i];
    }

    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = y[i];
        for k in (i + 1)..n {
            sum -= l[k * n + i] * x[k];
        }
        x[i] = sum / l[i * n + i];
    }

    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::residual::ConstraintResidual;
    use crate::variables::{VarId, VarSet};

    #[test]
    fn solves_simple_rectangle() {
        // c0(0,1) c1(2,3) c2(4,5) c3(6,7) — 80 x 60 rectangle at origin
        let eqs = vec![
            ConstraintResidual::FixedX {
                x: VarId(0),
                value: 0.0,
            },
            ConstraintResidual::FixedY {
                y: VarId(1),
                value: 0.0,
            },
            ConstraintResidual::Horizontal {
                x1: VarId(0),
                y1: VarId(1),
                x2: VarId(2),
                y2: VarId(3),
            },
            ConstraintResidual::Horizontal {
                x1: VarId(6),
                y1: VarId(7),
                x2: VarId(4),
                y2: VarId(5),
            },
            ConstraintResidual::Vertical {
                x1: VarId(0),
                y1: VarId(1),
                x2: VarId(6),
                y2: VarId(7),
            },
            ConstraintResidual::Vertical {
                x1: VarId(2),
                y1: VarId(3),
                x2: VarId(4),
                y2: VarId(5),
            },
            ConstraintResidual::Distance {
                x1: VarId(0),
                y1: VarId(1),
                x2: VarId(2),
                y2: VarId(3),
                target: 80.0,
            },
            ConstraintResidual::Distance {
                x1: VarId(0),
                y1: VarId(1),
                x2: VarId(6),
                y2: VarId(7),
                target: 60.0,
            },
        ];

        let vars = VarSet::new(vec![0.0, 0.0, 70.0, 5.0, 75.0, 58.0, 5.0, 55.0]);
        let out = gauss_newton_solve(&eqs, vars, &SolverOptions::default());
        assert!(out.converged, "max_error={}", out.max_error);
        assert!((out.vars.get(VarId(2)) - 80.0).abs() < 1e-4);
        assert!((out.vars.get(VarId(7)) - 60.0).abs() < 1e-4);
    }

    #[test]
    fn final_trial_error_controls_convergence_flag() {
        let eqs = vec![ConstraintResidual::FixedX {
            x: VarId(0),
            value: 1.0,
        }];
        let vars = VarSet::new(vec![0.0]);
        let options = SolverOptions {
            max_iterations: 1,
            tolerance: 1e-3,
            ..SolverOptions::default()
        };
        let out = gauss_newton_solve(&eqs, vars, &options);
        assert!(out.converged);
        assert!(out.max_error <= options.tolerance);
        assert!((out.max_error - (1e-4 / 1.0001)).abs() < 1e-8);
    }
}
