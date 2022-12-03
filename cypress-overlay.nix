# From: https://gist.github.com/r-k-b/2485f977b476aa3f76a47329ce7f9ad4

# When Cypress starts, it copies some files into `~/.config/Cypress/cy/production/browsers/chrome-stable/interactive/CypressExtension/`
# from the Nix Store, one of which it attempts to modify immediately after.
# As-is, this fails because the copied file keeps the read-only flag it had in
# the Store.
# Luckily, the code responsible is a plain text script that we can easily patch:
final: prev: {
  cypress = prev.cypress.overrideAttrs (oldAttrs: rec {
    pname = "cypress";
    version = "9.5.0";

    src = prev.fetchzip {
      url = "https://cdn.cypress.io/desktop/${version}/linux-x64/cypress.zip";
      sha256 = "1sjkrx8yv187waly1namcks013apmr73jsr0fpyqyp93ddri8lvc";
    };

    installPhase =
      let
        matchForChrome = "yield utils_1.default.copyExtension(pathToExtension, extensionDest);";
        appendForChrome = "yield fs_1.fs.chmodAsync(extensionBg, 0o0644);";

        matchForFirefox = "copyExtension(pathToExtension, extensionDest)";
        replaceForFirefox = "copyExtension(pathToExtension, extensionDest).then(() => fs.chmodAsync(extensionBg, 0o0644))";
      in
      ''
        sed -i '/${matchForChrome}/a\${appendForChrome}' \
            ./resources/app/packages/server/lib/browsers/chrome.js

        sed -i 's/${matchForFirefox}/${replaceForFirefox}/' \
            ./resources/app/packages/server/lib/browsers/utils.js
      '' + oldAttrs.installPhase;
  });
}