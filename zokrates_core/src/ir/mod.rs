use crate::flat_absy::flat_parameter::FlatParameter;
use crate::flat_absy::{FlatVariable, RuntimeError};
use crate::solvers::Solver;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;
use zokrates_field::Field;

mod expression;
pub mod folder;
pub mod from_flat;
mod interpreter;
mod serialize;
pub mod smtlib2;
pub mod visitor;
mod witness;

pub use self::expression::QuadComb;
pub use self::expression::{CanonicalLinComb, LinComb};
pub use self::serialize::ProgEnum;

pub use self::interpreter::{Error, ExecutionResult, Interpreter};
pub use self::witness::Witness;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq)]
pub enum Statement<T> {
    Constraint(QuadComb<T>, LinComb<T>, Option<RuntimeError>),
    Directive(Directive<T>),
    Log(String),
}

impl<T: Field> Statement<T> {
    pub fn definition<U: Into<QuadComb<T>>>(v: FlatVariable, e: U) -> Self {
        Statement::Constraint(e.into(), v.into(), None)
    }

    pub fn constraint<U: Into<QuadComb<T>>, V: Into<LinComb<T>>>(quad: U, lin: V) -> Self {
        Statement::Constraint(quad.into(), lin.into(), None)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Directive<T> {
    pub inputs: Vec<QuadComb<T>>,
    pub outputs: Vec<FlatVariable>,
    pub solver: Solver,
}

impl<T: Field> fmt::Display for Directive<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "# {} = {}({})",
            self.outputs
                .iter()
                .map(|o| format!("{}", o))
                .collect::<Vec<_>>()
                .join(", "),
            self.solver,
            self.inputs
                .iter()
                .map(|i| format!("{}", i))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl<T: Field> fmt::Display for Statement<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Statement::Constraint(ref quad, ref lin, _) => write!(f, "{} == {}", quad, lin),
            Statement::Directive(ref s) => write!(f, "{}", s),
            Statement::Log(ref s) => write!(f, "{}", s),
        }
    }
}

pub type Prog<T> = ProgIterator<T, Vec<Statement<T>>>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ProgIterator<T, I: IntoIterator<Item = Statement<T>>> {
    pub arguments: Vec<FlatParameter>,
    pub return_count: usize,
    pub statements: I,
}

impl<T, I: IntoIterator<Item = Statement<T>>> ProgIterator<T, I> {
    pub fn new(arguments: Vec<FlatParameter>, statements: I, return_count: usize) -> Self {
        Self {
            arguments,
            return_count,
            statements,
        }
    }

    pub fn collect(self) -> ProgIterator<T, Vec<Statement<T>>> {
        ProgIterator {
            statements: self.statements.into_iter().collect::<Vec<_>>(),
            arguments: self.arguments,
            return_count: self.return_count,
        }
    }

    pub fn returns(&self) -> Vec<FlatVariable> {
        (0..self.return_count).map(FlatVariable::public).collect()
    }

    pub fn public_count(&self) -> usize {
        self.arguments.iter().filter(|a| !a.private).count() + self.return_count
    }
}

impl<T: Field, I: IntoIterator<Item = Statement<T>>> ProgIterator<T, I> {
    pub fn public_inputs(&self, witness: &Witness<T>) -> Vec<T> {
        self.arguments
            .iter()
            .filter(|p| !p.private)
            .map(|p| witness.0.get(&p.id).unwrap().clone())
            .chain(witness.return_values())
            .collect()
    }
}

impl<T> Prog<T> {
    pub fn constraint_count(&self) -> usize {
        self.statements
            .iter()
            .filter(|s| matches!(s, Statement::Constraint(..)))
            .count()
    }

    pub fn into_prog_iter(self) -> ProgIterator<T, impl IntoIterator<Item = Statement<T>>> {
        ProgIterator {
            statements: self.statements.into_iter(),
            arguments: self.arguments,
            return_count: self.return_count,
        }
    }
}

impl<T: Field> fmt::Display for Prog<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "def main({}) -> ({}):",
            self.arguments
                .iter()
                .map(|v| format!("{}", v))
                .collect::<Vec<_>>()
                .join(", "),
            self.return_count,
        )?;
        for s in &self.statements {
            writeln!(f, "\t{}", s)?;
        }
        writeln!(
            f,
            "\treturn {}",
            (0..self.return_count)
                .map(FlatVariable::public)
                .map(|e| format!("{}", e))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zokrates_field::Bn128Field;

    mod statement {
        use super::*;

        #[test]
        fn print_constraint() {
            let c: Statement<Bn128Field> = Statement::Constraint(
                QuadComb::from_linear_combinations(
                    FlatVariable::new(42).into(),
                    FlatVariable::new(42).into(),
                ),
                FlatVariable::new(42).into(),
                None,
            );
            assert_eq!(format!("{}", c), "(1 * _42) * (1 * _42) == 1 * _42")
        }
    }
}
