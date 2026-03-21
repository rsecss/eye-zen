import MainApp from '../pages/main/MainApp.svelte';
import { mount } from 'svelte';

const app = mount(MainApp, { target: document.getElementById('app')! });

export default app;
