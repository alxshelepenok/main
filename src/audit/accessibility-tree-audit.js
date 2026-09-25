(function () {
  "use strict";
  if (!window.__SITE_AUDIT__) return;

  var SITE_URL = window.__SITE_AUDIT_URL__ || window.location.origin;

  var HEADING_TAGS = new Set(["H1", "H2", "H3", "H4", "H5", "H6"]);

  var LANDMARK_TAGS = new Set(["SECTION", "ARTICLE", "NAV", "ASIDE"]);

  var LANDMARK_ROLES = new Set([
    "region",
    "navigation",
    "complementary",
    "form",
    "search",
  ]);

  var PRESENTATION_ROLES = new Set(["presentation", "none"]);

  var INTERACTIVE_TAGS = new Set(["A", "BUTTON"]);

  var INTERACTIVE_ROLES = new Set(["link", "button"]);

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

  var fragmentOf = function (value) {
    if (typeof value !== "string") return undefined;
    var index = value.indexOf("#");
    return index >= 0 ? value.slice(index + 1) : undefined;
  };

  var textOf = function (el) { return (el.textContent || "").trim(); };

  var titleOf = function (el) { return (el.getAttribute("title") || "").trim(); };

  var isLandmark = function (el) {
    return (
      LANDMARK_TAGS.has(el.tagName) ||
      LANDMARK_ROLES.has(el.getAttribute("role") || "")
    );
  };

  var viewOf = function (doc) {
    return doc.defaultView || (typeof window !== "undefined" ? window : null);
  };

  var styleOf = function (view, el) {
    try {
      return view.getComputedStyle(el);
    } catch (error) {
      return null;
    }
  };

  var hiddenReason = function (el, doc) {
    var view = viewOf(doc);
    for (var node = el; node; node = node.parentElement) {
      var self = node === el;
      var where = self ? "it" : "an ancestor <" + node.tagName.toLowerCase() + ">";
      if (node.getAttribute("aria-hidden") === "true") {
        return where + ' has aria-hidden="true", which prunes the anchor from the accessibility tree';
      }
      if (node.hasAttribute("hidden")) {
        return where + " has the hidden attribute, removing the anchor from the accessibility tree";
      }
      var style = view ? styleOf(view, node) : null;
      if (style && style.display === "none") {
        return where + " is display:none, removing the anchor from the accessibility tree";
      }
      if (style && style.visibility === "hidden") {
        return where + " is visibility:hidden, removing the anchor from the accessibility tree";
      }
    }
    return undefined;
  };

  var duplicateId = function (id, doc) {
    var escaped =
      typeof CSS !== "undefined" && typeof CSS.escape === "function"
        ? CSS.escape(id)
        : id.replace(/[^\w-]/g, "\\$&");
    try {
      return doc.querySelectorAll("#" + escaped).length > 1;
    } catch (error) {
      return false;
    }
  };

  var ownHeading = function (el) {
    var headings = Array.prototype.slice.call(
      el.querySelectorAll("h1, h2, h3, h4, h5, h6, [role='heading']")
    );
    for (var i = 0; i < headings.length; i++) {
      var heading = headings[i];
      var nested = false;
      for (var p = heading.parentElement; p && p !== el; p = p.parentElement) {
        if (isLandmark(p)) {
          nested = true;
          break;
        }
      }
      if (!nested) return heading;
    }
    return null;
  };

  var INTERACTIVE_SELECTOR = "a[href], button, [role='link'], [role='button']";

  var LANDMARK_SELECTOR =
    "nav, aside, section, article, [role='region'], [role='navigation'], " +
    "[role='complementary'], [role='form'], [role='search']";

  var normalizeName = function (value) {
    return value
      .toLowerCase()
      .replace(/[^\p{L}\p{N} ]/gu, " ")
      .replace(/\s+/g, " ")
      .trim();
  };

  var labelledbyText = function (el, doc) {
    var value = el.getAttribute("aria-labelledby");
    if (value === null) return undefined;
    return value
      .split(/\s+/)
      .filter(Boolean)
      .map(function (id) {
        var ref = doc.getElementById(id);
        return ref ? textOf(ref) : "";
      })
      .join(" ")
      .trim();
  };

  var accessibleName = function (el, doc) {
    var ariaLabel = (el.getAttribute("aria-label") || "").trim();
    if (ariaLabel) return ariaLabel;

    var labelledby = labelledbyText(el, doc);
    if (labelledby) return labelledby;

    if (el.tagName === "IMG") {
      var alt = (el.getAttribute("alt") || "").trim();
      return alt || titleOf(el);
    }

    if (isLandmark(el)) {
      var heading = ownHeading(el);
      return heading ? textOf(heading) : titleOf(el);
    }

    return textOf(el) || titleOf(el);
  };

  var landmarkRole = function (el) {
    var role = el.getAttribute("role");
    if (role && LANDMARK_ROLES.has(role)) return role;
    switch (el.tagName) {
      case "NAV":
        return "navigation";
      case "ASIDE":
        return "complementary";
      case "SECTION":
        return "region";
      case "ARTICLE":
        return "article";
      default:
        return undefined;
    }
  };

  var locate = function (el) {
    if (el.id) return "#" + el.id;
    var href = el.getAttribute("href");
    if (href) return '[href="' + href + '"]';
    var cls = (el.getAttribute("class") || "").trim().split(/\s+/)[0];
    return cls ? "." + cls : el.tagName.toLowerCase();
  };

  var collectAnchors = function (nodes) {
    var anchors = new Set();
    var consider = function (value) {
      if (typeof value !== "string" || isExternal(value)) return;
      var fragment = fragmentOf(value);
      if (fragment) anchors.add(fragment);
    };
    nodes.forEach(function (node) {
      consider(node["@id"]);
      consider(node["url"]);
      var item = node["item"];
      if (item && typeof item === "object" && !Array.isArray(item)) {
        consider(item["@id"]);
        consider(item["url"]);
      } else {
        consider(item);
      }
    });
    return Array.from(anchors);
  };

  var collectAnchorNames = function (nodes) {
    var map = new Map();
    var add = function (value, name) {
      if (typeof value !== "string" || isExternal(value)) return;
      var fragment = fragmentOf(value);
      if (!fragment) return;
      if (typeof name !== "string" || !name.trim()) return;
      var list = map.get(fragment) || [];
      list.push(name.trim());
      map.set(fragment, list);
    };
    nodes.forEach(function (node) {
      add(node["@id"], node["name"]);
      add(node["url"], node["name"]);
      var item = node["item"];
      if (item && typeof item === "object" && !Array.isArray(item)) {
        var name = node["name"] !== undefined && node["name"] !== null ? node["name"] : item["name"];
        add(item["@id"], name);
        add(item["url"], name);
      }
    });
    return map;
  };

  var validateAnchor = function (anchor, doc, names) {
    var el = doc.getElementById(anchor);
    if (!el) {
      return [
        {
          anchor: anchor,
          tag: "",
          severity: "error",
          message: "anchor #" + anchor + " resolves to no element, the AI agent lands on nothing",
        },
      ];
    }

    var tag = el.tagName.toLowerCase();
    var role = el.getAttribute("role");
    var error = function (message) {
      return [{ anchor: anchor, tag: tag, severity: "error", message: message }];
    };
    var warn = function (message) {
      return [{ anchor: anchor, tag: tag, severity: "warn", message: message }];
    };

    if (duplicateId(anchor, doc)) {
      return error(
        'id "' + anchor + '" is not unique in the document, the anchor is ambiguous ' +
          "and the agent may resolve a different element than the schema intends"
      );
    }

    var pruned = hiddenReason(el, doc);
    if (pruned) return error("anchored <" + tag + "> is invisible to the agent: " + pruned);

    if (role !== null && PRESENTATION_ROLES.has(role)) {
      return error(
        'role="' + role + '" strips semantics, the anchored entity has no role in the accessibility tree'
      );
    }

    var interactive =
      INTERACTIVE_TAGS.has(el.tagName) || (role !== null && INTERACTIVE_ROLES.has(role));
    var heading = HEADING_TAGS.has(el.tagName) || role === "heading";
    var container = !interactive && !heading && el.tagName !== "IMG";

    var naming = function () {
      var ariaLabel = el.getAttribute("aria-label");
      if (ariaLabel !== null) {
        return ariaLabel.trim().length
          ? []
          : error("aria-label is present but empty, no programmatic name");
      }

      var labelledby = el.getAttribute("aria-labelledby");
      if (labelledby !== null) {
        var refs = labelledby.split(/\s+/).filter(Boolean);
        var missing = refs.filter(function (id) { return !doc.getElementById(id); });
        if (missing.length) {
          return error("aria-labelledby references missing id(s): " + missing.join(", "));
        }
        var text = refs
          .map(function (id) { return textOf(doc.getElementById(id)); })
          .join(" ")
          .trim();
        return text.length
          ? []
          : error("aria-labelledby resolves to empty text, no programmatic name");
      }

      if (el.tagName === "IMG") {
        var alt = el.getAttribute("alt");
        var title = titleOf(el);
        if (alt === null) {
          return title.length
            ? warn("<img> has no alt, it falls back to the fragile title attribute")
            : error("<img> is missing the alt attribute");
        }
        if (alt.trim().length) return [];
        return title.length
          ? warn("<img> has an empty alt and relies on the fragile title attribute")
          : error("<img> has an empty alt, an anchored image must be named");
      }

      if (heading) {
        return textOf(el).length ? [] : error("heading <" + tag + "> has no text content");
      }

      if (interactive) {
        if (textOf(el).length) return [];
        return titleOf(el).length
          ? warn("<" + tag + "> has no text and relies on the fragile title attribute")
          : error("<" + tag + "> has neither aria-label nor text content");
      }

      if (isLandmark(el)) {
        var own = ownHeading(el);
        if (!own) {
          return error(
            "landmark <" + tag + "> has no aria-label/aria-labelledby and contains " +
              "no heading (h1-h6) of its own, it stays an unnamed block in the accessibility tree"
          );
        }
        if (!textOf(own).length) {
          return error(
            "landmark <" + tag + "> contains an empty <" + own.tagName.toLowerCase() + ">, " +
              "it resolves to an unnamed block in the accessibility tree"
          );
        }
        return warn(
          "landmark <" + tag + "> is named only implicitly by its <" + own.tagName.toLowerCase() + "> " +
            '("' + textOf(own) + '"); bind it explicitly with aria-labelledby for a reliable name'
        );
      }

      if (textOf(el).length) return [];
      return titleOf(el).length
        ? warn("<" + tag + "> has no text and relies on the fragile title attribute")
        : error("<" + tag + "> has no programmatic name (no aria-label, no text content)");
    };

    var issues = naming();

    var expected = names.get(anchor) || [];
    if (expected.length) {
      var actual = accessibleName(el, doc);
      if (actual) {
        var target = normalizeName(actual);
        var matched = expected.some(function (value) {
          var candidate = normalizeName(value);
          return (
            candidate.length > 0 &&
            (candidate === target || candidate.includes(target) || target.includes(candidate))
          );
        });
        if (!matched) {
          issues = issues.concat(
            warn(
              'schema name "' + expected[0] + '" does not match the accessible name "' + actual + '", ' +
                "the agent may fail to map this element to the JSON-LD entity"
            )
          );
        }
      }
    }

    if (container) {
      Array.prototype.slice
        .call(el.querySelectorAll(INTERACTIVE_SELECTOR))
        .forEach(function (node) {
          if (hiddenReason(node, doc)) return;
          if (accessibleName(node, doc)) return;
          var childTag = node.tagName.toLowerCase();
          issues.push({
            anchor: anchor,
            tag: childTag,
            severity: "error",
            message:
              "descendant <" + childTag + "> " + locate(node) + " inside the anchored container has " +
              "no accessible name (text/aria-label), the agent cannot activate it",
          });
        });
    }

    return issues;
  };

  var validateLandmarkUniqueness = function (doc) {
    var issues = [];
    var groups = new Map();

    Array.prototype.slice.call(doc.querySelectorAll(LANDMARK_SELECTOR)).forEach(function (el) {
      if (hiddenReason(el, doc)) return;
      var role = landmarkRole(el);
      if (!role) return;
      var name = accessibleName(el, doc);
      var bareSection =
        (el.tagName === "SECTION" || el.tagName === "ARTICLE") &&
        !name &&
        el.getAttribute("role") === null;
      if (bareSection) return;
      var list = groups.get(role) || [];
      list.push({ el: el, name: name });
      groups.set(role, list);
    });

    groups.forEach(function (list, role) {
      if (list.length < 2) return;
      var counts = new Map();
      list.forEach(function (entry) {
        var key = normalizeName(entry.name);
        counts.set(key, (counts.get(key) || 0) + 1);
      });
      list.forEach(function (entry) {
        var key = normalizeName(entry.name);
        var tag = entry.el.tagName.toLowerCase();
        if (!key) {
          issues.push({
            anchor: entry.el.id,
            tag: tag,
            severity: "error",
            message:
              list.length + ' "' + role + '" landmarks coexist, but <' + tag + "> " + locate(entry.el) + " has " +
              "no accessible name; give sibling landmarks unique aria-labels so the agent can distinguish them",
          });
        } else if ((counts.get(key) || 0) > 1) {
          issues.push({
            anchor: entry.el.id,
            tag: tag,
            severity: "error",
            message:
              '"' + role + '" landmark name "' + entry.name + '" is shared by ' + counts.get(key) + " landmarks " +
              "(" + locate(entry.el) + "); give each a unique aria-label to preserve the user journey",
          });
        }
      });
    });

    return issues;
  };

  var auditAccessibilityTree = function (doc) {
    var collectGraph = window.__siteAuditCollectGraph;
    if (typeof collectGraph !== "function") {
      console.error("[audit-a11y] schema-audit.js is not loaded, cannot collect the graph");
      return [];
    }
    var nodes = collectGraph(doc);
    var names = collectAnchorNames(nodes);
    return []
      .concat(
        collectAnchors(nodes).flatMap(function (anchor) {
          return validateAnchor(anchor, doc, names);
        }),
        validateLandmarkUniqueness(doc)
      );
  };

  var reportA11yIssues = function (issues) {
    issues.forEach(function (issue) {
      var log = issue.severity === "warn" ? console.warn : console.error;
      var label = [issue.anchor ? "#" + issue.anchor : "", issue.tag ? "<" + issue.tag + ">" : ""]
        .filter(Boolean)
        .join(" ");
      log("[audit-a11y] " + label + ": " + issue.message);
    });
  };

  var run = function () { return reportA11yIssues(auditAccessibilityTree(document)); };
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", run, { once: true });
  } else {
    run();
  }
})();
