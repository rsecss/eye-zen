import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Toggle from '../Toggle.svelte';

describe('Toggle', () => {
  it('renders checked state', () => {
    render(Toggle, { props: { checked: true, onchange: vi.fn() } });
    expect(screen.getByRole('switch')).toHaveAttribute('aria-checked', 'true');
  });

  it('renders unchecked state', () => {
    render(Toggle, { props: { checked: false, onchange: vi.fn() } });
    expect(screen.getByRole('switch')).toHaveAttribute('aria-checked', 'false');
  });

  it('calls onchange with toggled value on click', async () => {
    const onchange = vi.fn();
    render(Toggle, { props: { checked: true, onchange } });
    await fireEvent.click(screen.getByRole('switch'));
    expect(onchange).toHaveBeenCalledWith(false);
  });

  it('calls onchange(true) when currently false', async () => {
    const onchange = vi.fn();
    render(Toggle, { props: { checked: false, onchange } });
    await fireEvent.click(screen.getByRole('switch'));
    expect(onchange).toHaveBeenCalledWith(true);
  });

  it('uses custom aria-label from label prop', () => {
    render(Toggle, { props: { checked: true, label: 'Sound', onchange: vi.fn() } });
    expect(screen.getByLabelText('Sound')).toBeInTheDocument();
  });
});
