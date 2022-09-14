export function shake(element: Element) {
  const keyframes = [
    { transform: "translate(1px, 1px) rotate(0deg)" },
    { transform: "translate(-1px, -2px) rotate(-1deg)" },
    { transform: "translate(-3px, 0px) rotate(1deg)" },
    { transform: "translate(3px, 2px) rotate(0deg)" },
    { transform: "translate(1px, -1px) rotate(1deg)" },
    { transform: "translate(-1px, 2px) rotate(-1deg)" },
    { transform: "translate(-3px, 1px) rotate(0deg)" },
    { transform: "translate(3px, 1px) rotate(-1deg)" },
    { transform: "translate(-1px, -1px) rotate(1deg)" },
    { transform: "translate(1px, 2px) rotate(0deg)" },
    { transform: "translate(1px, -2px) rotate(-1deg)" },
  ];
  const timing = { duration: 500, iterations: 1 };
  element.animate?.(keyframes, timing);
}
