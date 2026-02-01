import React from 'react';
import Button from './Button';

export default function Header({ title, onLogout }) {
  return (
    <header className="app-header">
      <h1>{title}</h1>
      <nav>
        <Button onClick={onLogout} variant="secondary">
          Logout
        </Button>
      </nav>
    </header>
  );
}
