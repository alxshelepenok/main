const randomNumber = (min, max) => {
 return Math.floor(Math.random() * (max - min + 1)) + min;
}

const splitWord = (word) => {
  return [...word]
    .map((letter) => {
      return `<span class="char">${letter}</span>`;
    })
    .join("");
};

class LinkedListNode {
  value = null;
  next = null;
  previous = null;

  constructor(value) {
    this.value = value;
  }
}

class LinkedList {
  head = null;
  tail = null;
  length = 0;

  constructor() {}

  add(value) {
    const node = new LinkedListNode(value);
    if (this.head === null) {
      this.head = node;
      this.tail = node;
    } else {
      this.tail.next = node;
      node.previous = this.tail;
      this.tail = node;
    }
    ++this.length;
    return node;
  }

  iterate(fn) {
    let node = this.head;
    let i = 0;
    while (node !== null) {
      fn(node, i);
      node = node.next;
      ++i;
    }
  }
}

class Cell {
  DOM = { el: null };
  position = -1;
  previousCellPosition = -1;
  original;
  state;

  constructor(el, { position, previousCellPosition } = {}) {
    this.DOM.el = el;
    this.original = this.DOM.el.innerHTML;
    this.state = this.original;
    this.position = position;
    this.previousCellPosition = previousCellPosition;
  }

  set(value) {
    this.state = value;
    this.DOM.el.innerHTML = this.state;
  }
}

class TypeShuffle {
  DOM = {
    el: null,
  };
  cells = [];
  lettersAndSymbols = [
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "!",
    "@",
    "#",
    "$",
    "&",
    "*",
    "(",
    ")",
    "-",
    "_",
    "+",
    "=",
    "/",
    "[",
    "]",
    "{",
    "}",
    ";",
    ":",
    "<",
    ">",
    ",",
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
  ];
  effects = {
    fx: () => this.fx(),
  };
  totalChars = 0;

  constructor(el) {
    this.DOM.el = el;

    let charCount = 0;
    const list = new LinkedList();

    [...window.atob(this.DOM.el.innerHTML).split(" ")].forEach((word) => {
      const wordEl = document.createElement("span");
      wordEl.className = "word";
      wordEl.innerHTML = splitWord(word);
      list.add(wordEl);
    });

    this.DOM.el.innerHTML = "";

    list.iterate((node) => {
      this.DOM.el.appendChild(node.value);
      if (node.next !== null) {
        this.DOM.el.appendChild(document.createTextNode(" "));
      }
      for (const char of [...node.value.querySelectorAll(".char")]) {
        this.cells.push(
          new Cell(char, {
            position: charCount,
            previousCellPosition: charCount === 0 ? -1 : charCount - 1,
          })
        );
        ++charCount;
      }
    });

    this.totalChars += charCount;
  }

  clearCells() {
    for (const cell of this.cells) {
      cell.set("&nbsp;");
    }
  }

  getRandomChar() {
    return this.lettersAndSymbols[
      Math.floor(Math.random() * this.lettersAndSymbols.length)
      ];
  }

  fx() {
    const MAX_CELL_ITERATIONS = 10;
    let finished = 0;
    this.clearCells();

    const loop = (cell, iteration = 0) => {
      if (iteration === MAX_CELL_ITERATIONS - 1) {
        cell.set(cell.original);
        ++finished;
        if (finished === this.totalChars) {
          this.isAnimating = false;
        }
      } else {
        cell.set(this.getRandomChar());
      }

      ++iteration;
      if (iteration < MAX_CELL_ITERATIONS) {
        setTimeout(() => loop(cell, iteration), 80);
      }
    };

    for (const cell of this.cells) {
      setTimeout(() => loop(cell), randomNumber(0, 1000));
    }
  }

  trigger(effect = "fx") {
    if (!(effect in this.effects) || this.isAnimating) return;
    this.isAnimating = true;
    this.effects[effect]();
  }
}

const btn = document.querySelector(".contact > button");

btn.addEventListener(
  "click",
  () => {
    const el = document.querySelector(".contact > #email");
    el.classList.add("decrypted");
    const ts = new TypeShuffle(el);
    ts.trigger("fx");
  },
  { once: true }
);
