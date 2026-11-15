// =============================================================================
// BILINGUAL DICTIONARY SHAPE & TYPED KEYS
// =============================================================================

import type { ErrorCode } from '#lib/api/types.generated.ts';

export type Locale = 'vi' | 'en';

export interface Dictionary {
  common: {
    appName: string;
    loading: string;
    save: string;
    saving: string;
    cancel: string;
    delete: string;
    confirm: string;
    edit: string;
    create: string;
    close: string;
    search: string;
    refresh: string;
    actions: string;
    empty: string;
    helpGuide: string;
    lightMode: string;
    darkMode: string;
    language: string;
    optionalStorageDisabled: string;
  };
  nav: {
    todos: string;
    workspaces: string;
    members: string;
    profile: string;
    settings: string;
    admin: string;
    signIn: string;
    signOut: string;
    activeWorkspace: string;
    notifications: string;
    markAllRead: string;
    noNotifications: string;
  };
  auth: {
    welcomeTitle: string;
    welcomeSubtitle: string;
    continueWithGoogle: string;
    testAccountLoginLink: string;
    testAccountTitle: string;
    testAccountSubtitle: string;
    email: string;
    password: string;
    signInButton: string;
    backToGoogleLogin: string;
    onboardingTitle: string;
    onboardingSubtitle: string;
    displayName: string;
    firstWorkspaceName: string;
    completeOnboarding: string;
    loginGateTitle: string;
    loginGateBody: string;
    hotSupportBanner: string;
    exitHotSupport: string;
    reauthTitle: string;
    reauthSubtitle: string;
    newPassword: string;
    verifyWithGoogle: string;
    reauthSuccessTitle: string;
    reauthSuccessBody: string;
  };
  todos: {
    pageTitle: string;
    pageSubtitle: string;
    guestBanner: string;
    addPlaceholder: string;
    descriptionPlaceholder: string;
    newTask: string;
    editTask: string;
    filterAll: string;
    statusTodo: string;
    statusInProgress: string;
    statusDone: string;
    priorityLow: string;
    priorityMedium: string;
    priorityHigh: string;
    priorityUrgent: string;
    dueDate: string;
    clearDueDate: string;
    assignee: string;
    unassigned: string;
    attachment: string;
    uploadAttachment: string;
    statsTotal: string;
    statsTodo: string;
    statsInProgress: string;
    statsDone: string;
    statsUrgent: string;
    noTodos: string;
    readOnlyPermission: string;
    createdSuccess: string;
    updatedSuccess: string;
    deletedSuccess: string;
  };
  workspaces: {
    pageTitle: string;
    pageSubtitle: string;
    createWorkspace: string;
    editWorkspace: string;
    workspaceName: string;
    workspaceSlug: string;
    workspaceDescription: string;
    workspaceColor: string;
    switchHere: string;
    activeBadge: string;
    ownerBadge: string;
    tasksCount: string;
    membersCount: string;
    membersTitle: string;
    membersSubtitle: string;
    invitePlaceholder: string;
    inviteButton: string;
    rolePreset: string;
    permissionsLabel: string;
    removeMember: string;
    cannotDeleteLast: string;
  };
  admin: {
    pageTitle: string;
    pageSubtitle: string;
    tabUsers: string;
    tabHotSupport: string;
    tabTestAccounts: string;
    tabNotifications: string;
    impersonate: string;
    closeHotSupport: string;
    createTestAccount: string;
    publishAnnouncement: string;
    targetAudience: string;
    htmlContent: string;
    active: string;
    inactive: string;
  };
  profile: {
    pageTitle: string;
    pageSubtitle: string;
    avatarLabel: string;
    securitySection: string;
    localPasswordNote: string;
  };
  settings: {
    pageTitle: string;
    pageSubtitle: string;
    appearanceSection: string;
    languageSection: string;
    sessionSection: string;
  };
  errors: Record<ErrorCode, string> & {
    NETWORK_ERROR: string;
    UNKNOWN_ERROR: string;
  };
}
