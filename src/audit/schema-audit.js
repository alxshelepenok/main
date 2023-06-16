(function () {
  "use strict";
  if (!window.__SITE_AUDIT__) return;

  var SITE_URL = window.__SITE_AUDIT_URL__ || window.location.origin;
  var SYSTEM_IMAGES_IDS = [];

  var SUPERIOR = new Set(["WebSite"]);

  var SUBORDINATE = new Set([
    "Brand",
    "OnlineBusiness",
    "ImageObject",
    "WebPage",
    "FAQPage",
    "Question",
    "Answer",
    "BreadcrumbList",
    "HowTo",
    "TechArticle",
    "CollectionPage",
    "ItemList",
    "ListItem",
    "Offer",
    "SoftwareApplication",
    "WebApplication",
    "DefinedTermSet",
    "SiteNavigationElement",
    "Blog",
    "BlogPosting",
    "ProfilePage",
  ]);

  var typesOf = function (node) {
    var value = node["@type"];
    return (Array.isArray(value) ? value : [value]).filter(function (entry) {
      return typeof entry === "string";
    });
  };

  var rankOf = function (node) {
    var types = typesOf(node);
    if (types.some(function (type) { return SUPERIOR.has(type); })) return "superior";
    if (types.some(function (type) { return SUBORDINATE.has(type); })) return "subordinate";
    return undefined;
  };

  var fragmentOf = function (value) {
    if (typeof value !== "string") return undefined;
    var index = value.indexOf("#");
    return index >= 0 ? value.slice(index + 1) : undefined;
  };

  var ABSOLUTE_URL_RE = /^https?:\/\//i;

  var isAbsoluteUrl = function (value) {
    return typeof value === "string" && ABSOLUTE_URL_RE.test(value);
  };

  var hasFileExtension = function (value) {
    try {
      var pathname = new URL(value).pathname;
      return /\.[a-z0-9]+$/i.test(pathname);
    } catch (error) {
      return false;
    }
  };

  var LIST_TYPES = new Set(["BreadcrumbList", "ItemList"]);

  var flatten = function (value, out) {
    if (Array.isArray(value)) {
      value.forEach(function (entry) { flatten(entry, out); });
    } else if (value && typeof value === "object") {
      var node = value;
      if (typesOf(node).length) out.push(node);
      Object.values(node).forEach(function (entry) { flatten(entry, out); });
    }
    return out;
  };

  var collectGraph = function (doc) {
    var nodes = [];
    doc.querySelectorAll('script[type="application/ld+json"]').forEach(function (script) {
      try {
        flatten(JSON.parse(script.textContent || "null"), nodes);
      } catch (error) {
        console.error("[audit-schema] unparseable JSON-LD block", script, error);
      }
    });
    return nodes;
  };

  var SKIP_EXTERNAL = true;

  var SITE_ORIGIN = new URL(SITE_URL).origin;

  var isExternal = function (value) {
    if (!SKIP_EXTERNAL) return false;
    try {
      var url = new URL(value, window.location.href);
      return url.origin !== SITE_ORIGIN || url.pathname !== window.location.pathname;
    } catch (error) {
      return false;
    }
  };

  var isSystemImage = function (node) {
    if (!typesOf(node).includes("ImageObject")) return false;
    var urls = [];
    if (typeof node.contentUrl === "string") urls.push(node.contentUrl);
    if (Array.isArray(node.thumbnailUrl)) {
      node.thumbnailUrl.forEach(function (u) {
        if (typeof u === "string") urls.push(u);
      });
    } else if (typeof node.thumbnailUrl === "string") {
      urls.push(node.thumbnailUrl);
    }
    return urls.some(function (url) {
      return SYSTEM_IMAGES_IDS.some(function (id) { return url.includes(id); });
    });
  };

  var validateNode = function (node, doc) {
    var rank = rankOf(node);
    if (!rank) return [];

    var type = typesOf(node).join(",");
    var types = typesOf(node);

    var id = node["@id"];
    var url = node["url"];

    if (types.includes("ListItem")) {
      var item = node.item;
      if (item && typeof item === "object" && !Array.isArray(item)) {
        id = item["@id"] !== undefined ? item["@id"] : id;
        url = item["url"] !== undefined ? item["url"] : url;
      } else if (typeof item === "string") {
        id = item;
        url = undefined;
      }
    }

    var issues = [];
    var fail = function (message) {
      issues.push({ type: type, id: typeof id === "string" ? id : undefined, message: message });
    };

    if (rank === "superior") {
      if (!node["name"]) fail("missing name");
      if (typeof id !== "string") fail("missing @id");
      return issues;
    }

    if (types.includes("ImageObject")) {
      var contentUrl = node.contentUrl;
      if (!isAbsoluteUrl(contentUrl)) {
        fail("ImageObject must have an absolute contentUrl (the file bytes)");
      } else if (!hasFileExtension(contentUrl)) {
        fail('contentUrl "' + contentUrl + '" must point to a file (e.g. .webp)');
      }

      if (isSystemImage(node)) {
        if (typeof url === "string" && url.length > 0) {
          fail("system image must not have url (no UI anchor, contentUrl only)");
        }
        return issues;
      }

      if (!isAbsoluteUrl(url)) {
        fail("ImageObject url must be an absolute UI anchor");
        return issues;
      }
    }

    if (typeof id !== "string") fail("missing @id");
    if (typeof url !== "string") {
      fail("missing url");
      return issues;
    }

    if (typeof id === "string" && isExternal(id)) return issues;
    if (isExternal(url)) return issues;

    var fragment = fragmentOf(url);
    if (!fragment) fail('url "' + url + '" has no #identifier');
    if (typeof id === "string" && id !== url) fail('@id "' + id + '" does not match url "' + url + '"');

    if (fragment && !doc.getElementById(fragment)) fail('no element with id="' + fragment + '"');

    return issues;
  };

  var validateMainEntityBreadcrumb = function (nodes) {
    var issues = [];
    var pageTypes = new Set(["WebPage", "FAQPage", "CollectionPage"]);

    nodes.forEach(function (node) {
      var types = typesOf(node);
      if (!types.some(function (t) { return pageTypes.has(t); })) return;

      var mainEntity = node.mainEntity;
      if (!mainEntity) return;

      var mainEntityId;
      if (typeof mainEntity === "string") {
        mainEntityId = mainEntity;
      } else if (Array.isArray(mainEntity)) {
        return;
      } else if (mainEntity && typeof mainEntity === "object") {
        mainEntityId = mainEntity["@id"];
      }
      if (!mainEntityId) return;

      var mainFragment = fragmentOf(mainEntityId);
      if (!mainFragment) {
        issues.push({
          type: types.join(","),
          id: typeof node["@id"] === "string" ? node["@id"] : undefined,
          message: 'mainEntity @id "' + mainEntityId + '" is missing a #fragment (semantic anchor required)',
        });
        return;
      }

      var breadcrumbs = nodes.filter(function (n) {
        return typesOf(n).includes("BreadcrumbList");
      });
      if (breadcrumbs.length === 0) return;

      breadcrumbs.forEach(function (breadcrumb) {
        var items = breadcrumb.itemListElement;
        if (!Array.isArray(items) || items.length === 0) return;

        var lastItem = items[items.length - 1];
        if (!lastItem || typeof lastItem !== "object") return;

        var lastUrl;
        var itemVal = lastItem.item;
        if (typeof itemVal === "string") {
          lastUrl = itemVal;
        } else if (itemVal && typeof itemVal === "object" && !Array.isArray(itemVal)) {
          lastUrl = itemVal.url;
        }
        if (!lastUrl) return;

        var lastFragment = fragmentOf(lastUrl);
        if (lastFragment !== mainFragment) {
          issues.push({
            type: "BreadcrumbList",
            id: typeof breadcrumb["@id"] === "string" ? breadcrumb["@id"] : undefined,
            message:
              'last breadcrumb item url "' + lastUrl + '" fragment #' +
              (lastFragment || "none") +
              " does not match mainEntity @id anchor #" + mainFragment,
          });
        }
      });
    });

    return issues;
  };

  var validateListPositions = function (nodes) {
    var issues = [];

    nodes.forEach(function (node) {
      var types = typesOf(node);
      if (!types.some(function (type) { return LIST_TYPES.has(type); })) return;

      var items = node.itemListElement;
      if (!Array.isArray(items)) return;

      var listType = types.find(function (type) { return LIST_TYPES.has(type); }) || types.join(",");
      var listId = typeof node["@id"] === "string" ? node["@id"] : undefined;
      var fail = function (message) {
        issues.push({ type: listType, id: listId, message: message });
      };

      items.forEach(function (entry, index) {
        var expected = index + 1;
        if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
          fail("itemListElement[" + index + "] is not a ListItem object");
          return;
        }

        if (!typesOf(entry).includes("ListItem")) {
          fail('itemListElement[' + index + '] is missing @type "ListItem"');
        }

        var position = entry.position;
        if (typeof position !== "number" || !Number.isInteger(position)) {
          fail('itemListElement[' + index + '] is missing an integer "position"');
          return;
        }

        if (position !== expected) {
          fail("ListItem position " + position + " is out of sequence (expected " + expected + ")");
        }
      });
    });

    return issues;
  };

  var FILE_EXTENSION_RE = /\.(txt|xml|md|png|ico|svg|webmanifest|jpe?g|webp|css|js|json)$/i;

  var validateInternalLinks = function (doc) {
    var issues = [];
    Array.prototype.slice.call(doc.querySelectorAll("a[href]")).forEach(function (el) {
      var raw = el.getAttribute("href");
      if (!raw) return;

      if (raw.charAt(0) === "#") {
        var fragment = raw.slice(1);
        if (fragment && !doc.getElementById(fragment)) {
          issues.push({
            type: "link",
            id: raw,
            message: 'same-page link "' + raw + '" resolves to no element with id="' + fragment + '"',
          });
        }
        return;
      }

      var url;
      try {
        url = new URL(raw, SITE_URL);
      } catch (error) {
        return;
      }
      if (url.origin !== SITE_ORIGIN) return;
      if (FILE_EXTENSION_RE.test(url.pathname)) return;
      if (!url.hash) {
        issues.push({
          type: "link",
          id: raw,
          message: 'internal link "' + raw + '" carries no fragment (expected #page style targets)',
        });
      }
    });
    return issues;
  };

  var auditGraph = function (doc) {
    var nodes = collectGraph(doc);
    return []
      .concat(
        nodes.flatMap(function (node) { return validateNode(node, doc); }),
        validateMainEntityBreadcrumb(nodes),
        validateListPositions(nodes),
        validateInternalLinks(doc)
      );
  };

  var reportIssues = function (issues) {
    issues.forEach(function (issue) {
      console.error("[audit-schema] " + issue.type + (issue.id ? " <" + issue.id + ">" : "") + ": " + issue.message);
    });
  };

  window.__siteAuditCollectGraph = collectGraph;

  var run = function () { return reportIssues(auditGraph(document)); };
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", run, { once: true });
  } else {
    run();
  }
})();
