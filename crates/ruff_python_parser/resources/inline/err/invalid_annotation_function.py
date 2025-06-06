def f[T]() -> (y := 3): ...
def g[T](arg: (x := 1)): ...
def h[T](x: (yield 1)): ...
def j[T]() -> (yield 1): ...
def l[T](x: (yield from 1)): ...
def n[T]() -> (yield from 1): ...
def p[T: (yield 1)](): ...      # yield in TypeVar bound
def q[T = (yield 1)](): ...     # yield in TypeVar default
def r[*Ts = (yield 1)](): ...   # yield in TypeVarTuple default
def s[**Ts = (yield 1)](): ...  # yield in ParamSpec default
def t[T: (x := 1)](): ...       # named expr in TypeVar bound
def u[T = (x := 1)](): ...      # named expr in TypeVar default
def v[*Ts = (x := 1)](): ...    # named expr in TypeVarTuple default
def w[**Ts = (x := 1)](): ...   # named expr in ParamSpec default
