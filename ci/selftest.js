// Test automatique de MD Lite, injecté dans la page quand MDLITE_SELFTEST est défini (voir ci/run-linux.sh).
// Chaque étape est signalée au script de la CI par un fichier <étape>.out dans le dossier MDLITE_SELFTEST.
(() => {
  if (location.pathname !== '/') return;
  const log = [];
  let fails = 0;
  const check = (name, ok, info = '') => {
    log.push((ok ? 'ok   ' : 'FAIL ') + name + (info ? ' : ' + info : ''));
    if (!ok) fails++;
  };
  const step = (name, body = '', extra = '') => fetch('/api/selftest?step=' + name + extra, { method: 'POST', body });
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const until = async (f, ms = 20000) => {
    for (const t0 = Date.now(); Date.now() - t0 < ms; await sleep(100)) {
      try { if (f()) return true } catch (e) {}
    }
    return false;
  };
  const tab = name => tabs.find(t => t.name === name);

  addEventListener('load', async () => {
    try {
      // Démarrage : le fichier passé en argument et le guide du premier lancement.
      check('démarrage', await until(() => window.__mdlite && tab('test.md') && tab('guide.md')));
      check('système détecté', OS === 'linux', OS);
      const t = tab('test.md');
      activate(t);
      setMode('split', false);

      // Rendu Markdown.
      check('titre', out.querySelector('h1')?.textContent === 'Titre de test');
      check('coloration du code', out.querySelectorAll('pre [class^="hl-"]').length >= 3);
      check('note de bas de page', !!out.querySelector('.fnref') && !!out.querySelector('.footnotes'));
      check('case à cocher', !!out.querySelector('input[type=checkbox]'));
      check('tableau', out.querySelectorAll('table td').length === 4);
      const img = out.querySelector('img');
      check('image relative', await until(() => img.complete && img.naturalWidth > 0, 10000), img && img.src);

      // Bandeau « application par défaut » puis association avec les fichiers .md.
      check('bandeau par défaut affiché', await until(() => !$('banner').hidden, 5000));
      $('bdef').click();
      check('définie par défaut', await until(() => $('banner').hidden, 10000));
      check('xdg-mime', (await (await fetch('/api/default')).text()) === '1');

      // Modification puis enregistrement à la même place.
      ta.value += '\nLigne ajoutée par le test.\n';
      changed();
      check('onglet modifié', t.dirty && document.title.startsWith('●'), document.title);
      await save(false, t);
      check('enregistré', !t.dirty, document.title);

      // Paramètres enregistrés dans ~/.config/md-lite.
      const st = JSON.parse(await (await fetch('/api/store?key=settings')).text());
      check('paramètres enregistrés', st.welcomed === true);

      // Instance unique : la CI lance un second md-lite, qui confie son fichier à cette fenêtre.
      await step('ready');
      check('instance unique', await until(() => tab('second.md') && T === tab('second.md')));
      check('onglets', tabs.length === 3, tabs.map(t => t.name).join(', '));

      // Boîtes de dialogue GTK : la CI fait une capture puis appuie sur Échap.
      activate(t);
      ta.value += 'x';
      changed();
      await step('dialog');
      check('dialogue « Enregistrer ? »', (await (await fetch('/api/confirmclose?name=test.md', { method: 'POST' })).text()) === 'cancel');
      ta.value = ta.value.slice(0, -1);
      await save(false, t);
      await step('pick');
      check('dialogue d\'ouverture', (await (await fetch('/api/pick?doc=' + t.id, { method: 'POST' })).text()) === '[]');

      // Thème sombre.
      setSetting('theme', 'dark');
      await sleep(300);
      check('thème sombre', document.documentElement.dataset.theme === 'dark');
      await step('dark');
      await sleep(1500);
      setSetting('theme', 'system');
    } catch (e) {
      check('exception', false, e && (e.stack || e.message || e));
    }
    await step('result', log.join('\n'));
    // Fermeture normale (session enregistrée) si tout va bien.
    if (fails) step('done', '', '&ok=0');
    else __mdlite.requestClose();
  });
})();
