import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Stepper from '../Stepper.svelte';

describe('Stepper', () => {
  const defaults = {
    value: 20,
    min: 1,
    max: 120,
    step: 1,
    unit: 'min',
    label: 'Work',
    onchange: vi.fn(),
  };

  it('renders current value with unit', () => {
    render(Stepper, { props: defaults });
    expect(screen.getByText('20')).toBeInTheDocument();
    expect(screen.getByText('min')).toBeInTheDocument();
  });

  it('increments on + click', async () => {
    const onchange = vi.fn();
    render(Stepper, { props: { ...defaults, onchange } });
    await fireEvent.click(screen.getByLabelText('Increase Work'));
    expect(onchange).toHaveBeenCalledWith(21);
  });

  it('decrements on − click', async () => {
    const onchange = vi.fn();
    render(Stepper, { props: { ...defaults, onchange } });
    await fireEvent.click(screen.getByLabelText('Decrease Work'));
    expect(onchange).toHaveBeenCalledWith(19);
  });

  it('clamps to max', async () => {
    const onchange = vi.fn();
    render(Stepper, { props: { ...defaults, value: 120, onchange } });
    await fireEvent.click(screen.getByLabelText('Increase Work'));
    expect(onchange).not.toHaveBeenCalled();
  });

  it('clamps to min', async () => {
    const onchange = vi.fn();
    render(Stepper, { props: { ...defaults, value: 1, onchange } });
    await fireEvent.click(screen.getByLabelText('Decrease Work'));
    expect(onchange).not.toHaveBeenCalled();
  });

  it('uses step size correctly', async () => {
    const onchange = vi.fn();
    render(Stepper, { props: { ...defaults, value: 20, step: 5, unit: 'sec', onchange } });
    await fireEvent.click(screen.getByLabelText('Increase Work'));
    expect(onchange).toHaveBeenCalledWith(25);
  });

  it('uses unique aria-labels with label prop', () => {
    render(Stepper, { props: { ...defaults, label: 'Rest Duration' } });
    expect(screen.getByLabelText('Decrease Rest Duration')).toBeInTheDocument();
    expect(screen.getByLabelText('Increase Rest Duration')).toBeInTheDocument();
  });

  it('does not call onchange when disabled', async () => {
    const onchange = vi.fn();
    render(Stepper, { props: { ...defaults, disabled: true, onchange } });
    await fireEvent.click(screen.getByLabelText('Increase Work'));
    await fireEvent.click(screen.getByLabelText('Decrease Work'));
    expect(onchange).not.toHaveBeenCalled();
  });
});
