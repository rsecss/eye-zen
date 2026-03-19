import TipApp from '../pages/tip/TipApp.svelte';
import { mount } from 'svelte';

const app = mount(TipApp, { target: document.getElementById('app')! });

export default app;
