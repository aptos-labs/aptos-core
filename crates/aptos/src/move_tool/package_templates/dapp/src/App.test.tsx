import React from 'react';
import { render, screen } from '@testing-library/react';
import App from './App';

test('renders check balance button', () => {
  render(<App />);
  const buttonElement = screen.getByText(/Check Balance/i);
  expect(buttonElement).toBeInTheDocument();
});
