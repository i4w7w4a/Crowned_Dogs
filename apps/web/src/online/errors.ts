import { OnlineError } from "./transport.ts";

export const isLoginExpired = (error: unknown): boolean => error instanceof OnlineError
  && ["login_expired", "browser_login_expired"].includes(error.message);

export const onlineErrorMessage = (error: unknown): string =>
  error instanceof OnlineError && error.message === "mini_app_reopen" ? "Вход Telegram истёк. Закройте игру и откройте её заново из Telegram." :
  error instanceof OnlineError && error.status === 401 ? "Сессия истекла. Обновите страницу и войдите через Telegram." :
  error instanceof OnlineError && error.message === "busy" ? "Профиль закреплён за вызовом или боем. Сначала завершите бой или отмените вызов." :
  isLoginExpired(error) ? "Время подтверждения истекло. Начните вход заново." :
  error instanceof OnlineError && error.status === 409 ? "Состояние уже изменилось. Данные обновлены; повторите выбор." :
  "Сервер недоступен. Проверьте соединение и повторите.";
