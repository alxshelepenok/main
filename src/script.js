const randomNumber = (min, max) =>
  Math.floor(Math.random() * (max - min + 1)) + min;

const splitWord = (word) =>
  [...word].map((letter) => `<span class="char">${letter}</span>`).join("");

const characterIsSupported = (
  character,
  font = getComputedStyle(document.body).fontFamily,
  recursion = false,
) => {
  const testCanvas = document.createElement("canvas");
  const referenceCanvas = document.createElement("canvas");
  testCanvas.width =
    referenceCanvas.width =
      testCanvas.height =
        referenceCanvas.height =
          150;

  const testContext = testCanvas.getContext("2d");
  const referenceContext = referenceCanvas.getContext("2d");
  testContext.font = referenceContext.font = "100px " + font;
  testContext.fillStyle = referenceContext.fillStyle = "black";
  testContext.fillText(character, 0, 100);
  referenceContext.fillText("\uffff", 0, 100);

  if (!recursion && characterIsSupported("\ufffe", font, true)) {
    testContext.fillStyle = referenceContext.fillStyle = "black";
    testContext.fillRect(10, 10, 80, 80);
    referenceContext.fillRect(10, 10, 80, 80);
  }

  return testCanvas.toDataURL() !== referenceCanvas.toDataURL();
};

const createCell = (el, { position, previousCellPosition } = {}) => ({
  DOM: { el },
  original: el.innerHTML,
  state: el.innerHTML,
  position,
  previousCellPosition,
  set(value) {
    this.state = value;
    this.DOM.el.innerHTML = this.state;
  },
});

const initTypeShuffle = (el) => {
  const cells = [];
  let lettersAndSymbols = ["A", "B", ..."9"];

  if (characterIsSupported("ラ")) {
    lettersAndSymbols = lettersAndSymbols.concat(["ラ", "ド", ..."ウ"]);
  }

  const clearCells = () => cells.forEach((cell) => cell.set("&nbsp;"));

  const getRandomChar = () =>
    lettersAndSymbols[Math.floor(Math.random() * lettersAndSymbols.length)];

  const fx = () => {
    const MAX_CELL_ITERATIONS = 7;
    let finished = 0;
    clearCells();

    const loop = (cell, iteration = 0) => {
      if (iteration === MAX_CELL_ITERATIONS - 1) {
        cell.set(cell.original);
        finished++;
      } else {
        cell.set(getRandomChar());
      }

      iteration++;
      if (iteration < MAX_CELL_ITERATIONS) {
        setTimeout(() => loop(cell, iteration), 80);
      }
    };

    cells.forEach((cell) =>
      setTimeout(() => loop(cell), randomNumber(0, 1000)),
    );
  };

  const triggerEffect = () => {
    const words = window.atob(el.innerHTML).split(" ");
    el.innerHTML = "";

    words.forEach((word) => {
      const wordEl = document.createElement("span");
      wordEl.className = "word";
      wordEl.innerHTML = splitWord(word);
      el.appendChild(wordEl);
      el.appendChild(document.createTextNode(" "));

      [...wordEl.querySelectorAll(".char")].forEach((charEl, index) => {
        cells.push(
          createCell(charEl, {
            position: index,
            previousCellPosition: index - 1,
          }),
        );
      });
    });

    fx();
  };

  return { triggerEffect };
};

const shuffleElement = document.querySelector(".contact > #email");
const typeShuffle = initTypeShuffle(shuffleElement);

document.querySelector(".contact > button").addEventListener(
  "click",
  () => {
    typeShuffle.triggerEffect();
    shuffleElement.classList.add("decrypted");
  },
  { once: true },
);
