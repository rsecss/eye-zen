import { render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { describe, expect, it } from 'vitest';
import SettingsCard from '../SettingsCard.svelte';

const children = createRawSnippet(() => ({
  render: () => '<span>Card content</span>',
}));

describe('SettingsCard', () => {
  it('renders its title and child content', () => {
    render(SettingsCard, { props: { title: 'Timer', children } });

    expect(screen.getByText('Timer')).toBeInTheDocument();
    expect(screen.getByText('Card content')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('renders a user-visible error when provided', () => {
    render(SettingsCard, { props: { title: 'Timer', error: 'Save failed', children } });

    expect(screen.getByRole('alert')).toHaveTextContent('Save failed');
  });
});
