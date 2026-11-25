// Azeradio hub. Cards render from playlists.json. No cards are hardcoded here.

function normalize(s) {
  return (s || '').toLowerCase();
}

// Subsequence match, kept to one word: every character of the query
// appears in a single word of the name, in order, but not necessarily
// next to each other. That is what lets "str" match both Stranglethorn
// Vale and Stormwind, without also matching Dustwallow Marsh (whose
// s and t sit in "dustwallow" and whose r sits in "marsh"). A query
// that spans words, like "thunder bluff", matches as a plain substring
// of the whole name instead.
function matchesQuery(name, query) {
  var q = normalize(query).replace(/\s+/g, '');
  if (!q) return true;
  var squashed = normalize(name).replace(/\s+/g, '');
  if (squashed.indexOf(q) !== -1) return true;
  var words = normalize(name).split(/\s+/);
  for (var w = 0; w < words.length; w++) {
    var word = words[w];
    var i = 0;
    for (var k = 0; k < word.length; k++) {
      if (word[k] === q[i]) i++;
      if (i === q.length) return true;
    }
  }
  return false;
}

function filterPlaylists(list, query, group) {
  return list.filter(function (p) {
    if (group !== 'All' && p.group !== group) return false;
    return matchesQuery(p.name, query);
  });
}

// Private playlists get no embed: a private embed renders "not found"
// to visitors, so they only get the Open in Spotify link.
function shouldEmbed(p) {
  return p.public === true;
}

function embedUrl(p) {
  return 'https://open.spotify.com/embed/playlist/' + p.spotifyId + '?utm_source=generator&theme=0';
}

function openUrl(p) {
  return 'https://open.spotify.com/playlist/' + p.spotifyId;
}

if (typeof document !== 'undefined') {
  document.addEventListener('DOMContentLoaded', init);
}

function init() {
  var grid = document.getElementById('grid');
  var empty = document.getElementById('empty');
  var search = document.getElementById('search');
  var count = document.getElementById('count');
  var chips = Array.prototype.slice.call(document.querySelectorAll('.chip'));

  var all = [];
  var query = '';
  var group = 'All';
  var openSlug = null;

  fetch('playlists.json')
    .then(function (r) { return r.json(); })
    .then(function (list) { all = list; render(); })
    .catch(function () {
      empty.hidden = false;
      empty.textContent = 'Could not load the playlists.';
    });

  function render() {
    var filtered = filterPlaylists(all, query, group);
    openSlug = null;
    grid.textContent = '';

    filtered.forEach(function (p) {
      grid.appendChild(buildCard(p));
    });

    empty.hidden = filtered.length !== 0;
    if (filtered.length === 0) {
      empty.textContent = 'Nothing matches. Try another zone.';
    }

    var filtering = query !== '' || group !== 'All';
    count.textContent = filtering
      ? filtered.length + (filtered.length === 1 ? ' match' : ' matches')
      : all.length + (all.length === 1 ? ' playlist' : ' playlists');
  }

  function buildCard(p) {
    var card = document.createElement('article');
    card.className = 'card';
    card.setAttribute('data-slug', p.slug);

    var img = document.createElement('img');
    img.src = 'covers/' + p.cover;
    img.alt = 'Pixel art cover for ' + p.name;
    img.loading = 'lazy';
    card.appendChild(img);

    var title = document.createElement('h3');
    title.textContent = p.name;
    card.appendChild(title);

    var essence = document.createElement('p');
    essence.className = 'essence';
    essence.textContent = p.essence;
    card.appendChild(essence);

    var player = document.createElement('div');
    player.className = 'player';
    card.appendChild(player);

    if (shouldEmbed(p)) {
      var play = document.createElement('button');
      play.type = 'button';
      play.className = 'btn btn-primary';
      play.textContent = 'Play here';
      play.addEventListener('click', function () { togglePlayer(p, card, player, play); });
      card.appendChild(play);

      card.tabIndex = 0;
      card.addEventListener('click', function (e) {
        if (e.target.closest('a, button, iframe')) return;
        togglePlayer(p, card, player, play);
      });
      card.addEventListener('keydown', function (e) {
        if ((e.key === 'Enter' || e.key === ' ') && e.target === card) {
          e.preventDefault();
          togglePlayer(p, card, player, play);
        }
      });
    } else {
      var note = document.createElement('p');
      note.className = 'private-note';
      note.textContent = 'Private for now. Listen in Spotify.';
      card.appendChild(note);
    }

    var open = document.createElement('a');
    open.className = 'btn' + (shouldEmbed(p) ? '' : ' btn-primary');
    open.href = openUrl(p);
    open.target = '_blank';
    open.rel = 'noopener';
    open.textContent = 'Open in Spotify';
    card.appendChild(open);

    return card;
  }

  function togglePlayer(p, card, player, play) {
    if (openSlug === p.slug) {
      closePlayer(card, player, play);
      return;
    }
    var openCard = grid.querySelector('.card.is-open');
    if (openCard) {
      closePlayer(
        openCard,
        openCard.querySelector('.player'),
        openCard.querySelector('.btn-primary')
      );
    }
    var frame = document.createElement('iframe');
    frame.title = 'Spotify playlist: Azeroth: ' + p.name;
    frame.src = embedUrl(p);
    frame.loading = 'lazy';
    frame.allow = 'autoplay; clipboard-write; encrypted-media; fullscreen; picture-in-picture';
    frame.allowFullscreen = true;
    player.appendChild(frame);
    card.classList.add('is-open');
    play.textContent = 'Close player';
    openSlug = p.slug;
  }

  function closePlayer(card, player, play) {
    player.textContent = '';
    card.classList.remove('is-open');
    if (play) play.textContent = 'Play here';
    openSlug = null;
  }

  search.addEventListener('input', function () {
    query = search.value;
    render();
  });

  chips.forEach(function (chip) {
    chip.addEventListener('click', function () {
      group = chip.getAttribute('data-group');
      chips.forEach(function (c) {
        var active = c === chip;
        c.classList.toggle('is-active', active);
        c.setAttribute('aria-pressed', active ? 'true' : 'false');
      });
      render();
    });
  });

  document.addEventListener('keydown', function (e) {
    var tag = (e.target && e.target.tagName) || '';
    if (e.key === '/' && tag !== 'INPUT' && tag !== 'TEXTAREA' && tag !== 'SELECT') {
      e.preventDefault();
      search.focus();
    }
    if (e.key === 'Escape' && search.value !== '') {
      search.value = '';
      query = '';
      search.blur();
      render();
    }
  });
}
