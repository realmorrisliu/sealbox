import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from './button';
import { Globe, ChevronDown } from 'lucide-react';

export function LanguageSelector() {
  const { i18n, t } = useTranslation();
  const [isOpen, setIsOpen] = useState(false);

  const languages = [
    { code: 'en', name: t('language.english'), flag: '🇺🇸' },
    { code: 'zh', name: t('language.chinese'), flag: '🇨🇳' },
    { code: 'ja', name: t('language.japanese'), flag: '🇯🇵' },
    { code: 'de', name: t('language.german'), flag: '🇩🇪' },
  ];

  const currentLanguage = languages.find(lang => lang.code === i18n.language) || languages[0];

  const handleLanguageChange = (languageCode: string) => {
    i18n.changeLanguage(languageCode);
    setIsOpen(false);
  };

  return (
    <div className="relative">
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setIsOpen(!isOpen)}
        className="gap-2"
        title={t('language.switch')}
      >
        <Globe className="h-4 w-4" />
        <span className="text-sm hidden sm:inline">{currentLanguage.name}</span>
        <span className="text-sm sm:hidden">{currentLanguage.flag}</span>
        <ChevronDown className="h-3 w-3" />
      </Button>

      {isOpen && (
        <div className="absolute right-0 top-full mt-1 bg-card border rounded-md shadow-lg z-50 min-w-[140px]">
          {languages.map((language) => (
            <button
              key={language.code}
              onClick={() => handleLanguageChange(language.code)}
              className={`w-full px-3 py-2 text-sm text-left hover:bg-muted flex items-center gap-2 ${
                language.code === i18n.language ? 'bg-muted' : ''
              }`}
            >
              <span>{language.flag}</span>
              <span>{language.name}</span>
            </button>
          ))}
        </div>
      )}

      {isOpen && (
        <div 
          className="fixed inset-0 z-40" 
          onClick={() => setIsOpen(false)}
        />
      )}
    </div>
  );
}