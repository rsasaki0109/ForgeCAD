use crate::dof::{estimate_dof_generic, rank_of_jacobian};
use crate::jacobian::{finite_difference_jacobian_generic, Jacobian};
use crate::numeric::{gauss_newton_solve_generic, SolveOutput, SolverOptions};
use crate::residual::{evaluate_residuals_generic, ConstraintResidual, ResidualEquation};
use crate::variables::VarSet;

/// A residual must exceed this multiple of the configured tolerance before a
/// rank-inconsistent system is reported as contradictory.  The margin avoids
/// treating finite-difference noise just above the solve tolerance as a hard
/// modeling conflict.
pub const CONTRADICTION_ERROR_MULTIPLIER: f64 = 10.0;

/// Solver outcome with DOF and redundancy diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub enum SolveStatus {
    Solved {
        iterations: usize,
        max_error: f64,
    },
    UnderConstrained {
        dof: i32,
        iterations: usize,
        max_error: f64,
    },
    OverConstrained {
        redundant: usize,
        iterations: usize,
        max_error: f64,
    },
    /// The residual is outside the configured tolerance and is not in the
    /// column space of the Jacobian.  At least one over-constraining equation
    /// therefore conflicts with the independent constraint rows.
    Contradictory {
        redundant: usize,
        message: String,
        iterations: usize,
        max_error: f64,
    },
    /// The numeric iteration budget was exhausted (or a non-finite residual
    /// was encountered) without proving a contradiction.
    NonConvergent {
        dof: i32,
        message: String,
        iterations: usize,
        max_error: f64,
    },
    /// Retained for source compatibility with callers that used the former
    /// catch-all failure variant.  New diagnostics use [`Self::NonConvergent`]
    /// or [`Self::Contradictory`] instead.
    Failed {
        message: String,
        iterations: usize,
        max_error: f64,
    },
}

impl SolveStatus {
    pub fn is_solved(&self) -> bool {
        matches!(self, Self::Solved { .. })
    }
}

/// Solve and classify the result.
pub fn solve_with_diagnostics(
    equations: &[ConstraintResidual],
    vars: VarSet,
    options: &SolverOptions,
) -> (SolveOutput, SolveStatus) {
    solve_with_diagnostics_generic(equations, vars, options)
}

/// Solve and classify the result for any [`ResidualEquation`] type.
pub fn solve_with_diagnostics_generic<E: ResidualEquation>(
    equations: &[E],
    vars: VarSet,
    options: &SolverOptions,
) -> (SolveOutput, SolveStatus) {
    let output = gauss_newton_solve_generic(equations, vars, options);
    let dof = estimate_dof_generic(equations, &output.vars);

    let jacobian = finite_difference_jacobian_generic(equations, &output.vars);
    let rank = rank_of_jacobian(&jacobian);
    let redundant = equations.len().saturating_sub(rank);
    let contradictory = !output.converged
        && output.max_error.is_finite()
        && output.max_error > options.tolerance * CONTRADICTION_ERROR_MULTIPLIER
        && augmented_rank_exceeds_jacobian(equations, &output.vars, &jacobian, rank);

    let status = if output.converged {
        if dof > 0 {
            SolveStatus::UnderConstrained {
                dof,
                iterations: output.iterations,
                max_error: output.max_error,
            }
        } else if redundant > 0 {
            SolveStatus::OverConstrained {
                redundant,
                iterations: output.iterations,
                max_error: output.max_error,
            }
        } else {
            SolveStatus::Solved {
                iterations: output.iterations,
                max_error: output.max_error,
            }
        }
    } else if contradictory {
        SolveStatus::Contradictory {
            redundant,
            message: format!(
                "constraints are contradictory: max_error={} exceeds contradiction threshold={} (tolerance={} × {})",
                output.max_error,
                options.tolerance * CONTRADICTION_ERROR_MULTIPLIER,
                options.tolerance,
                CONTRADICTION_ERROR_MULTIPLIER,
            ),
            iterations: output.iterations,
            max_error: output.max_error,
        }
    } else {
        SolveStatus::NonConvergent {
            dof,
            message: format!(
                "solver did not converge within {} iterations: max_error={} exceeds tolerance={}",
                options.max_iterations, output.max_error, options.tolerance
            ),
            iterations: output.iterations,
            max_error: output.max_error,
        }
    };

    (output, status)
}

/// Return whether the linearized residual system is inconsistent.
///
/// A residual vector `r` is locally satisfiable when it lies in the column
/// space of `J`.  Appending it as one column and finding a larger rank is the
/// deterministic rank test for the contrary case (`rank([J | r]) > rank(J)`).
fn augmented_rank_exceeds_jacobian<E: ResidualEquation>(
    equations: &[E],
    vars: &VarSet,
    jacobian: &Jacobian,
    jacobian_rank: usize,
) -> bool {
    // A zero Jacobian only says that the current linearization has no usable
    // direction; it cannot prove that a nonlinear system has no solution.
    // Leave such cases in the explicit non-convergent category.
    if equations.is_empty() || jacobian_rank == 0 {
        return false;
    }
    let residuals = evaluate_residuals_generic(equations, vars);
    if residuals.iter().any(|residual| !residual.is_finite()) {
        return false;
    }

    let mut augmented = Jacobian::new(jacobian.rows, jacobian.cols + 1);
    for (row, residual) in residuals.iter().enumerate().take(jacobian.rows) {
        for col in 0..jacobian.cols {
            augmented.set(row, col, jacobian.get(row, col));
        }
        augmented.set(row, jacobian.cols, *residual);
    }

    rank_of_jacobian(&augmented) > jacobian_rank
}

/// Detect obviously duplicate equations (same type and variables).
///
/// This legacy one-argument helper has no variable values and therefore cannot
/// perform a Jacobian rank calculation.  Use
/// [`crate::dof::count_redundant_equations_generic`] for diagnostic counts.
pub fn count_redundant_equations(equations: &[ConstraintResidual]) -> usize {
    let mut keys = Vec::new();
    let mut redundant = 0_usize;
    for eq in equations {
        let key = format!("{eq:?}");
        if keys.contains(&key) {
            redundant += 1;
        } else {
            keys.push(key);
        }
    }
    redundant
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::residual::ConstraintResidual;
    use crate::variables::{VarId, VarSet};

    #[test]
    fn classifies_under_constrained_system() {
        let eqs = vec![ConstraintResidual::FixedX {
            x: VarId(0),
            value: 0.0,
        }];
        let vars = VarSet::new(vec![0.0, 0.0]);
        let (_, status) = solve_with_diagnostics(&eqs, vars, &SolverOptions::default());
        assert!(matches!(status, SolveStatus::UnderConstrained { .. }));
    }

    #[test]
    fn detects_duplicate_equations() {
        let eqs = vec![
            ConstraintResidual::FixedX {
                x: VarId(0),
                value: 0.0,
            },
            ConstraintResidual::FixedX {
                x: VarId(0),
                value: 0.0,
            },
        ];
        assert_eq!(count_redundant_equations(&eqs), 1);
    }

    #[test]
    fn classifies_fully_constrained_system() {
        let eqs = vec![ConstraintResidual::FixedX {
            x: VarId(0),
            value: 2.0,
        }];
        let vars = VarSet::new(vec![0.0]);
        let (_, status) = solve_with_diagnostics(&eqs, vars, &SolverOptions::default());
        assert!(matches!(status, SolveStatus::Solved { .. }));
    }

    #[test]
    fn classifies_rank_redundant_system() {
        let eqs = vec![
            ConstraintResidual::FixedX {
                x: VarId(0),
                value: 2.0,
            },
            ConstraintResidual::FixedX {
                x: VarId(0),
                value: 2.0,
            },
        ];
        let vars = VarSet::new(vec![0.0]);
        let (output, status) =
            solve_with_diagnostics(&eqs, vars.clone(), &SolverOptions::default());
        assert!(output.converged);
        assert!(matches!(
            status,
            SolveStatus::OverConstrained { redundant: 1, .. }
        ));
        assert_eq!(
            crate::dof::count_redundant_equations_at(&eqs, &output.vars),
            1
        );
    }

    #[test]
    fn classifies_contradictory_over_constraint() {
        let eqs = vec![
            ConstraintResidual::FixedX {
                x: VarId(0),
                value: 0.0,
            },
            ConstraintResidual::FixedX {
                x: VarId(0),
                value: 1.0,
            },
        ];
        let vars = VarSet::new(vec![0.0]);
        let (output, status) = solve_with_diagnostics(&eqs, vars, &SolverOptions::default());
        assert!(!output.converged);
        let SolveStatus::Contradictory { message, .. } = status else {
            panic!("expected contradictory status")
        };
        assert!(message.contains("contradiction threshold="));
        assert!(message.contains("tolerance="));
    }

    #[test]
    fn classifies_nonconvergence_without_contradiction() {
        let eqs = vec![ConstraintResidual::FixedX {
            x: VarId(0),
            value: 1.0,
        }];
        let vars = VarSet::new(vec![0.0]);
        let options = SolverOptions {
            max_iterations: 0,
            ..SolverOptions::default()
        };
        let (output, status) = solve_with_diagnostics(&eqs, vars, &options);
        assert!(!output.converged);
        let SolveStatus::NonConvergent { message, .. } = status else {
            panic!("expected non-convergent status")
        };
        assert!(message.contains("within 0 iterations"));
        assert!(message.contains("tolerance="));
    }
}
