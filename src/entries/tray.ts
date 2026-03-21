import TrayApp from '../pages/tray/TrayApp.svelte';
import { mount } from 'svelte';

const app = mount(TrayApp, { target: document.getElementById('app')! });

export default app;
