use errors::*;
use ir::*;
use solve::environment::InEnvironment;
use solve::implemented_with_clause::ImplementedWithClause;
use solve::implemented_with_impl::ImplementedWithImpl;
use solve::solver::Solver;
use solve::Solution;

pub struct Implemented<'s> {
    solver: &'s mut Solver,
    env_goal: Quantified<InEnvironment<TraitRef>>,
}

impl<'s> Implemented<'s> {
    pub fn new(solver: &'s mut Solver, env_goal: Quantified<InEnvironment<TraitRef>>) -> Self {
        Implemented {
            solver: solver,
            env_goal: env_goal,
        }
    }

    pub fn solve(self) -> Result<Solution<Quantified<InEnvironment<TraitRef>>>> {
        let Implemented { solver, env_goal } = self;
        let program = solver.program.clone();

        // First try to find a solution in the environment.
        let environment = &env_goal.value.environment;
        let num_clauses = environment.clauses.len();
        let env_result = solver.solve_any(0..num_clauses, &env_goal, |solver, clause_index| {
            ImplementedWithClause::new(solver, env_goal.clone(), clause_index).solve()
        });

        // If we found something in the environment, even if it might
        // have caused us to infer things that are not necessarily
        // correct, then take it. This is not obviously the right
        // thing to do but it models rustc's current behavior.
        if let Ok(v) = env_result {
            return Ok(v);
        }

        // Nothing in the environment, so try impls.
        solver.solve_any(&program.impl_data, &env_goal, |solver, (&impl_id, impl_data)| {
            // screen out the things that are for the totally wrong
            // trait, just to keep debug logging under control
            let goal_trait_id = env_goal.value.goal.trait_id;
            if impl_data.trait_ref.trait_id != goal_trait_id {
                bail!("impl for wrong trait");
            }

            ImplementedWithImpl::new(solver, env_goal.clone(), impl_id).solve()
        }).chain_err(|| {
            format!("`{:?}` is not implemented in environment `{:?}`",
                    env_goal.value.goal,
                    env_goal.value.environment)
        })
    }
}