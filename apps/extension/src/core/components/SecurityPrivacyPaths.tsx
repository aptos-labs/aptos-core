import { Routes } from 'core/routes';

export type SecurityPrivacyItem = {
  id: number;
  label: string;
  path: string;
};

const SecurityPrivacyPaths = () => {
  const items: SecurityPrivacyItem[] = [
    {
      id: 1,
      label: 'Change password',
      path: Routes.change_password.path,
    },
  ];

  return items;
};

export default SecurityPrivacyPaths;
