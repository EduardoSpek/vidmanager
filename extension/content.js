(function () {
  function detectMedia() {
    let count = 0;

    document.querySelectorAll("video").forEach((video) => {
      if (video.duration || video.currentSrc || video.src) count += 1;
    });

    if (document.querySelector('meta[property="og:video"], meta[property="og:video:url"]')) {
      count += 1;
    }

    const links = document.querySelectorAll('a[href$=".mp4"], a[href*=".m3u8"], a[href*=".mpd"]');
    if (links.length > 0) count += 1;

    return count;
  }

  function report() {
    try {
      chrome.runtime.sendMessage({
        type: "MEDIA_COUNT",
        count: detectMedia(),
        title: document.title
      });
    } catch {
      /* contexto invalidado */
    }
  }

  chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    if (message?.type === "GET_INFO") {
      sendResponse({
        title: document.title,
        mediaCount: detectMedia(),
        url: location.href
      });
    }
  });

  report();
  window.setTimeout(report, 2500);
})();
