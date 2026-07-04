import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'

export default createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', component: HomeView },
    { path: '/generation/:id', component: () => import('../views/GenerationView.vue') },
    { path: '/new', component: () => import('../views/NewPostView.vue') },
    { path: '/projects', component: () => import('../views/ProjectsView.vue') },
    { path: '/profiles', component: () => import('../views/ProfilesView.vue') },
    { path: '/settings', component: () => import('../views/SettingsView.vue') },
  ],
})
