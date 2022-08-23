// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export default function indexColor(colorIndex: number): string {
  const index = colorIndex % 10;
  switch (index) {
    case 0:
      return 'cyan';
    case 1:
      return 'purple';
    case 2:
      return 'blue';
    case 3:
      return 'green';
    case 4:
      return 'yellow';
    case 5:
      return 'orange';
    case 6:
      return 'red';
    case 7:
      return 'gray';
    case 8:
      return 'teal';
    case 9:
      return 'pink';
    default:
      return 'teal';
  }
}
