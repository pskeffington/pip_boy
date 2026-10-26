"use strict";
// Runs inside an exact-size Chromium iframe using production dashboard assets.
// The test server supplies synthetic profile/module state and no live transport.
(() => {
  const $ = q => document.querySelector(q);
  const all = q => [...document.querySelectorAll(q)];
  const shown = e => !!e && getComputedStyle(e).display !== "none" &&
    !e.closest("[hidden]") && e.getClientRects().length > 0;
  const inside = (e, host) => {
    if (!shown(e) || !shown(host)) return false;
    const a=e.getBoundingClientRect(), b=host.getBoundingClientRect();
    return a.top >= b.top-2 && a.bottom <= b.bottom+2 &&
      a.left >= b.left-2 && a.right <= b.right+2;
  };
  const documentFits = () => document.documentElement.scrollHeight <= innerHeight+1 &&
    document.documentElement.scrollWidth <= innerWidth+1 &&
    document.body.scrollHeight <= innerHeight+1;
  function report() {
    const result={width:innerWidth, height:innerHeight, documentFits:documentFits()};
    try {
      const roles=$("#role-list"),modules=$("#module-grid");
      result.initial={
        deckFits:shown($('[data-stage="modules"]')) &&
          inside($('[data-stage="modules"]'),$("#workspace-stage")),
        moduleCount:all(".module-card").length,
        cardsFit:all(".module-card").every(e=>inside(e,modules)),
        roleCount:all(".role-card").length,
        rolesFit:all(".role-card").every(e=>inside(e,roles)),
        pagerFits:all("#module-pager button").every(e=>inside(e,$("#module-pager"))),
        paneTabsVisible:innerWidth>1100 || shown($('[data-pane-target="workspace"]'))
      };
      if($("#module-pager button:last-child")){
        $("#module-pager button:last-child").click();
        result.nextModules={
          count:all(".module-card").length,
          countLabel:$("#module-pager .deck-count")?.textContent,
          fit:all(".module-card").every(e=>inside(e,$("#module-grid")))
        };
      }
      if($(".module-card")){
        $(".module-card").click();
        result.inspector=innerWidth>1100 ||
          (shown($(".detail-panel")) && shown($("#inspector-return")));
        if(innerWidth<=1100)$("#inspector-return").click();
      }
      $('[data-stage-target="viewport"]').click();
      result.viewport=shown($('[data-stage="viewport"]')) &&
        inside($('[data-stage="viewport"]'),$("#workspace-stage"));
      $('[data-stage-target="messages"]').click();
      result.messages=shown($('[data-stage="messages"]')) &&
        inside($('[data-stage="messages"]'),$("#workspace-stage"));
      const customTab=$('[data-stage-target="palette"]');
      if(shown(customTab)){
        customTab.click();
        result.palette={
          visible:shown($('[data-stage="palette"]')),
          count:all(".palette-button").length,
          fit:all(".palette-button").every(e=>inside(e,$("#palette-grid")))
        };
        if($("#palette-pager button:last-child")){
          $("#palette-pager button:last-child").click();
          result.palette.next=all(".palette-button").length;
        }
      }
      const roleTab=$('[data-pane-target="roles"]');
      if(shown(roleTab))roleTab.click();
      result.rolePane=shown($(".role-panel")) &&
        all(".role-card").every(e=>inside(e,roles));
      result.documentFitsAfter=documentFits();
      result.ok=true;
    }catch(e){result.error=e.stack || e.message;}
    parent.postMessage({type:"dashboard-viewport",value:result},location.origin);
  }
  let tries=0;
  function poll(){
    if((all(".role-card").length && $("#server-status").textContent.includes("READY")) ||
        ++tries>120)report();
    else setTimeout(poll,25);
  }
  window.addEventListener("load",poll);
})();