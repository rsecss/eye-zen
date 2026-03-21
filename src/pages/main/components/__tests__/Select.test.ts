import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Select from '../Select.svelte';

const options = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'en', label: 'English' },
];

describe('Select', () => {
  it('renders with current value selected', () => {
    render(Select, { props: { value: 'zh-CN', options, onchange: vi.fn() } });
    const select = screen.getByRole('combobox') as HTMLSelectElement;
    expect(select.value).toBe('zh-CN');
  });

  it('renders all options', () => {
    render(Select, { props: { value: 'zh-CN', options, onchange: vi.fn() } });
    const opts = screen.getAllByRole('option');
    expect(opts).toHaveLength(2);
  });

  it('calls onchange on selection', async () => {
    const onchange = vi.fn();
    render(Select, { props: { value: 'zh-CN', options, onchange } });
    await fireEvent.change(screen.getByRole('combobox'), { target: { value: 'en' } });
    expect(onchange).toHaveBeenCalledWith('en');
  });

  it('uses custom aria-label from label prop', () => {
    render(Select, { props: { value: 'zh-CN', options, label: 'Language', onchange: vi.fn() } });
    expect(screen.getByLabelText('Language')).toBeInTheDocument();
  });
});
