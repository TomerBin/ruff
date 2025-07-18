use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::helpers::is_compound_statement;
use ruff_python_ast::{self as ast, Expr, Stmt, WithItem};
use ruff_python_semantic::SemanticModel;
use ruff_text_size::Ranged;

use crate::Violation;
use crate::checkers::ast::Checker;
use crate::registry::Rule;

use crate::rules::flake8_pytest_style::helpers::is_empty_or_null_string;

/// ## What it does
/// Checks for `pytest.raises` context managers with multiple statements.
///
/// This rule allows `pytest.raises` bodies to contain `for`
/// loops with empty bodies (e.g., `pass` or `...` statements), to test
/// iterator behavior.
///
/// ## Why is this bad?
/// When a `pytest.raises` is used as a context manager and contains multiple
/// statements, it can lead to the test passing when it actually should fail.
///
/// A `pytest.raises` context manager should only contain a single simple
/// statement that raises the expected exception.
///
/// ## Example
/// ```python
/// import pytest
///
///
/// def test_foo():
///     with pytest.raises(MyError):
///         setup()
///         func_to_test()  # not executed if `setup()` raises `MyError`
///         assert foo()  # not executed
/// ```
///
/// Use instead:
/// ```python
/// import pytest
///
///
/// def test_foo():
///     setup()
///     with pytest.raises(MyError):
///         func_to_test()
///     assert foo()
/// ```
///
/// ## References
/// - [`pytest` documentation: `pytest.raises`](https://docs.pytest.org/en/latest/reference/reference.html#pytest-raises)
#[derive(ViolationMetadata)]
pub(crate) struct PytestRaisesWithMultipleStatements;

impl Violation for PytestRaisesWithMultipleStatements {
    #[derive_message_formats]
    fn message(&self) -> String {
        "`pytest.raises()` block should contain a single simple statement".to_string()
    }
}

/// ## What it does
/// Checks for `pytest.raises` calls without a `match` parameter.
///
/// ## Why is this bad?
/// `pytest.raises(Error)` will catch any `Error` and may catch errors that are
/// unrelated to the code under test. To avoid this, `pytest.raises` should be
/// called with a `match` parameter. The exception names that require a `match`
/// parameter can be configured via the
/// [`lint.flake8-pytest-style.raises-require-match-for`] and
/// [`lint.flake8-pytest-style.raises-extend-require-match-for`] settings.
///
/// ## Example
/// ```python
/// import pytest
///
///
/// def test_foo():
///     with pytest.raises(ValueError):
///         ...
///
///     # empty string is also an error
///     with pytest.raises(ValueError, match=""):
///         ...
/// ```
///
/// Use instead:
/// ```python
/// import pytest
///
///
/// def test_foo():
///     with pytest.raises(ValueError, match="expected message"):
///         ...
/// ```
///
/// ## Options
/// - `lint.flake8-pytest-style.raises-require-match-for`
/// - `lint.flake8-pytest-style.raises-extend-require-match-for`
///
/// ## References
/// - [`pytest` documentation: `pytest.raises`](https://docs.pytest.org/en/latest/reference/reference.html#pytest-raises)
#[derive(ViolationMetadata)]
pub(crate) struct PytestRaisesTooBroad {
    exception: String,
}

impl Violation for PytestRaisesTooBroad {
    #[derive_message_formats]
    fn message(&self) -> String {
        let PytestRaisesTooBroad { exception } = self;
        format!(
            "`pytest.raises({exception})` is too broad, set the `match` parameter or use a more \
             specific exception"
        )
    }
}

/// ## What it does
/// Checks for `pytest.raises` calls without an expected exception.
///
/// ## Why is this bad?
/// `pytest.raises` expects to receive an expected exception as its first
/// argument. If omitted, the `pytest.raises` call will fail at runtime.
///
/// ## Example
/// ```python
/// import pytest
///
///
/// def test_foo():
///     with pytest.raises():
///         do_something()
/// ```
///
/// Use instead:
/// ```python
/// import pytest
///
///
/// def test_foo():
///     with pytest.raises(SomeException):
///         do_something()
/// ```
///
/// ## References
/// - [`pytest` documentation: `pytest.raises`](https://docs.pytest.org/en/latest/reference/reference.html#pytest-raises)
#[derive(ViolationMetadata)]
pub(crate) struct PytestRaisesWithoutException;

impl Violation for PytestRaisesWithoutException {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Set the expected exception in `pytest.raises()`".to_string()
    }
}

pub(crate) fn is_pytest_raises(func: &Expr, semantic: &SemanticModel) -> bool {
    semantic
        .resolve_qualified_name(func)
        .is_some_and(|qualified_name| matches!(qualified_name.segments(), ["pytest", "raises"]))
}

const fn is_non_trivial_with_body(body: &[Stmt]) -> bool {
    if let [stmt] = body {
        is_compound_statement(stmt)
    } else {
        true
    }
}

pub(crate) fn raises_call(checker: &Checker, call: &ast::ExprCall) {
    if is_pytest_raises(&call.func, checker.semantic()) {
        if checker.is_rule_enabled(Rule::PytestRaisesWithoutException) {
            if call
                .arguments
                .find_argument("expected_exception", 0)
                .is_none()
            {
                checker.report_diagnostic(PytestRaisesWithoutException, call.func.range());
            }
        }

        if checker.is_rule_enabled(Rule::PytestRaisesTooBroad) {
            // Pytest.raises has two overloads
            // ```py
            // with raises(expected_exception: type[E] | tuple[type[E], ...], *, match: str | Pattern[str] | None = ...) → RaisesContext[E] as excinfo
            // with raises(expected_exception: type[E] | tuple[type[E], ...], func: Callable[[...], Any], *args: Any, **kwargs: Any) → ExceptionInfo[E] as excinfo
            // ```
            // Don't raise this diagnostic if the call matches the second overload (has a second positional argument or an argument named `func`)
            if call.arguments.find_argument("func", 1).is_none() {
                if let Some(exception) = call.arguments.find_argument_value("expected_exception", 0)
                {
                    if call
                        .arguments
                        .find_keyword("match")
                        .is_none_or(|k| is_empty_or_null_string(&k.value))
                    {
                        exception_needs_match(checker, exception);
                    }
                }
            }
        }
    }
}

pub(crate) fn complex_raises(checker: &Checker, stmt: &Stmt, items: &[WithItem], body: &[Stmt]) {
    let raises_called = items.iter().any(|item| match &item.context_expr {
        Expr::Call(ast::ExprCall { func, .. }) => is_pytest_raises(func, checker.semantic()),
        _ => false,
    });

    // Check body for `pytest.raises` context manager
    if raises_called {
        let is_too_complex = if let [stmt] = body {
            match stmt {
                Stmt::With(ast::StmtWith { body, .. }) => is_non_trivial_with_body(body),
                // Allow function and class definitions to test decorators.
                Stmt::ClassDef(_) | Stmt::FunctionDef(_) => false,
                // Allow empty `for` loops to test iterators.
                Stmt::For(ast::StmtFor { body, .. }) => match &body[..] {
                    [Stmt::Pass(_)] => false,
                    [Stmt::Expr(ast::StmtExpr { value, .. })] => !value.is_ellipsis_literal_expr(),
                    _ => true,
                },
                stmt => is_compound_statement(stmt),
            }
        } else {
            true
        };

        if is_too_complex {
            checker.report_diagnostic(PytestRaisesWithMultipleStatements, stmt.range());
        }
    }
}

/// PT011
fn exception_needs_match(checker: &Checker, exception: &Expr) {
    if let Some(qualified_name) = checker
        .semantic()
        .resolve_qualified_name(exception)
        .and_then(|qualified_name| {
            let qualified_name = qualified_name.to_string();
            checker
                .settings
                .flake8_pytest_style
                .raises_require_match_for
                .iter()
                .chain(
                    &checker
                        .settings
                        .flake8_pytest_style
                        .raises_extend_require_match_for,
                )
                .any(|pattern| pattern.matches(&qualified_name))
                .then_some(qualified_name)
        })
    {
        checker.report_diagnostic(
            PytestRaisesTooBroad {
                exception: qualified_name,
            },
            exception.range(),
        );
    }
}
