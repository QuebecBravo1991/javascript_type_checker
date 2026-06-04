function fac(n) {
  var f;
  if (n == 0) {
    f = 1;
  } else {
    f = fac(n - 1) * n;
  }
  return f;
}
let result = fac(5);
