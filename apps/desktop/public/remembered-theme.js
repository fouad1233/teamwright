// Sets the theme the window remembered on the root before anything is
// drawn, so that the window does not open in Lamplight and then turn
// (decision 0028). It runs before the interface's own code, which checks
// the name and asks the backend for the theme it keeps. A name no style
// sheet knows draws as Lamplight.
try {
  var remembered = window.localStorage.getItem('teamwright.theme');
  if (remembered && remembered !== 'lamplight' && /^[a-z]+$/.test(remembered)) {
    document.documentElement.setAttribute('data-theme', remembered);
  }
} catch {
  // Without storage the window opens in Lamplight.
}
