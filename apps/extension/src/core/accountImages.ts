// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export default function indexImage(styleIndex: number): string {
  const index = styleIndex % 14;
  switch (index) {
    case 0:
      return 'acid-dark-1.png';
    case 1:
      return 'acid-dark-2.png';
    case 2:
      return 'acid-dark-3.png';
    case 3:
      return 'acid-dark-4.png';
    case 4:
      return 'acid-dark-5.png';
    case 5:
      return 'acid-dark-6.png';
    case 6:
      return 'acid-dark-7.png';
    case 7:
      return 'acid-dark-8.png';
    case 8:
      return 'acid-dark-9.png';
    case 9:
      return 'acid-dark-10.png';
    case 10:
      return 'acid-dark-11.png';
    case 11:
      return 'acid-dark-12.png';
    case 12:
      return 'acid-dark-13.png';
    case 13:
      return 'acid-dark-14.png';
    default:
      return 'acid-dark-1.png';
  }
}
