import {
  ArrowLeft,
  BookOpenCheck,
  BrainCircuit,
  CalendarClock,
  FileText,
  ListChecks,
  PenLine,
  SkipForward,
  Timer,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { MINUTES_PER_TOPIC } from "@/lib/study";

/**
 * What the app does, in the order the student meets it.
 *
 * Shown once straight after setup — with a skip, because someone who already knows what a
 * spaced-repetition study app is should not have to read this — and available from the
 * header afterwards.
 */
export function GuideScreen({
  onDone,
  firstRun,
}: {
  onDone: () => void;
  /** After setup the button reads as "start"; from the header it reads as "back". */
  firstRun: boolean;
}) {
  return (
    <div className="mx-auto w-full max-w-3xl space-y-8 p-8">
      <header className="space-y-3">
        <div className="flex items-center justify-between">
          {firstRun ? (
            <span className="text-xs uppercase tracking-wide text-muted-foreground">
              Перший запуск
            </span>
          ) : (
            <Button variant="ghost" size="sm" onClick={onDone} className="-ml-2">
              <ArrowLeft className="size-4" />
              Назад
            </Button>
          )}
          {/* Reachable without scrolling: someone who already knows what a
              spaced-repetition study app is should not have to read to the bottom. */}
          {firstRun ? (
            <Button variant="ghost" size="sm" onClick={onDone}>
              Пропустити
              <SkipForward className="size-4" />
            </Button>
          ) : null}
        </div>
        <h1 className="text-2xl font-semibold tracking-tight">Як це працює</h1>
        <p className="text-sm leading-relaxed text-muted-foreground">
          Застосунок перетворює перелік екзаменаційних питань на власну базу конспектів,
          проводить вас по них сесіями й запам'ятовує, що саме ви забуваєте.
        </p>
      </header>

      <section className="space-y-3">
        <Step
          number={1}
          icon={<FileText className="size-5 text-primary" />}
          title="Силабус — те, що ви даєте самі"
          body="Один markdown-файл на предмет у теці syllabus/: заголовок, розділи через ##, під ними нумерований перелік тем. Ім'я файлу стає ідентифікатором предмета. Приклад уже лежить у сховищі."
        />
        <Step
          number={2}
          icon={<BrainCircuit className="size-5 text-primary" />}
          title="Конспекти — генеруються один раз"
          body="На вкладці «Теми» розумна модель пише конспект для кожної теми: суть, відповідь одним абзацом, механізм, ключові терміни й типові пастки. Це найдорожчий крок, тому кожен конспект зберігається на диску й більше не перегенеровується. Процес можна перервати — зроблене не втрачається."
        />
        <Step
          number={3}
          icon={<BookOpenCheck className="size-5 text-primary" />}
          title="Сесія — те, заради чого все інше"
          body="2–6 тем за раз. Спершу читаєте конспекти, потім письмово відповідаєте на одне широке питання з кожної теми — як на екзамені — і закріплюєте міні-квізом."
        />
        <Step
          number={4}
          icon={<CalendarClock className="size-5 text-primary" />}
          title="Оцінка й повторення"
          body="Письмові відповіді перевіряє розумна модель за переліком очікуваних пунктів: що розкрито, що пропущено, чого бракує. Оцінка теми — 60% за письмову частину, 40% за квіз."
        />
      </section>

      <section className="space-y-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <PenLine className="size-4 text-muted-foreground" />
          Темп сесії
        </h2>
        <Card>
          <CardContent className="overflow-x-auto p-0">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-left text-xs uppercase tracking-wide text-muted-foreground">
                  <th className="px-4 py-2.5 font-medium">Темп</th>
                  <th className="px-4 py-2.5 font-medium">Читання</th>
                  <th className="px-4 py-2.5 font-medium">Відповідь</th>
                  <th className="px-4 py-2.5 text-right font-medium">Хв/тема</th>
                </tr>
              </thead>
              <tbody>
                <Pace mode="full" label="Повний" reading="весь конспект" answer="кілька абзаців" />
                <Pace mode="balanced" label="Збалансований" reading="весь конспект" answer="короткі пункти" />
                <Pace mode="sprint" label="Спринт" reading="стислий конспект" answer="короткі пункти" />
              </tbody>
            </table>
          </CardContent>
        </Card>
        <p className="text-xs leading-relaxed text-muted-foreground">
          Скорочується спершу відповідь, і лише потім читання: конспект, якого ви не бачили,
          згадати неможливо, тому «Збалансований» віддає есе, а не текст. Питання завжди
          складаються рівно з того, що вам показали — спринт не спитає про те, чого не було в
          стислій версії.
        </p>
      </section>

      <section className="space-y-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <ListChecks className="size-4 text-muted-foreground" />
          Добір тем
        </h2>
        <div className="grid gap-3 sm:grid-cols-3">
          <Tile
            title="Автоматично"
            body="Спершу прострочені повторення, далі нові теми за порядком силабуса."
          />
          <Tile
            title="Як на екзамені"
            body="По одній темі з кожної частини силабуса, з ухилом до слабших — так, як витягують білет."
          />
          <Tile title="Вручну" body="Ви самі знаєте, що саме питатимуть завтра." />
        </div>
      </section>

      <section className="space-y-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <CalendarClock className="size-4 text-muted-foreground" />
          Сходинка повторень
        </h2>
        <Card>
          <CardContent className="space-y-3 py-5 text-sm leading-relaxed">
            <div className="flex flex-wrap items-center gap-2">
              {[1, 2, 4, 7, 14, 30].map((days, index) => (
                <span key={days} className="flex items-center gap-2">
                  {index > 0 ? <span className="text-border">→</span> : null}
                  <Badge variant="outline" className="tabular-nums">
                    {days} дн.
                  </Badge>
                </span>
              ))}
            </div>
            <p className="text-muted-foreground">
              80% і вище — тема піднімається на сходинку вгору, і наступне повторення
              відсувається. 60–79% — залишається на місці. Менше 60% — опускається на
              сходинку вниз і повернеться завтра.
            </p>
          </CardContent>
        </Card>
      </section>

      <section className="space-y-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <Timer className="size-4 text-muted-foreground" />
          Дрібниці, які варто знати
        </h2>
        <ul className="space-y-2 text-sm leading-relaxed text-muted-foreground">
          <li>
            Сесія записується на диск одразу, тому вихід посеред неї нічого не втрачає —
            повернутись можна кнопкою «Продовжити». Написане зберігається під час набору.
          </li>
          <li>
            Таймер у шапці рахує підходи по 25 хвилин і нагадує про перерву. Він нічого не
            перериває.
          </li>
          <li>
            «Моделі» у шапці — вибір моделі для кожного рівня. Конспекти та підказки завжди
            йдуть розумною, тести й сесії — швидкою.
          </li>
          <li>
            Все, що застосунок пише, лежить у вашій теці звичайними файлами: конспекти —
            markdown, решта — JSON.
          </li>
        </ul>
      </section>

      <div className="flex justify-end border-t border-border pt-6">
        <Button onClick={onDone}>{firstRun ? "Зрозуміло, почнімо" : "Закрити"}</Button>
      </div>
    </div>
  );
}

function Step({
  number,
  icon,
  title,
  body,
}: {
  number: number;
  icon: React.ReactNode;
  title: string;
  body: string;
}) {
  return (
    <Card>
      <CardContent className="flex gap-4 py-5">
        <div className="flex flex-col items-center gap-2">
          {icon}
          <span className="font-mono text-xs text-muted-foreground">{number}</span>
        </div>
        <div className="min-w-0 flex-1 space-y-1">
          <p className="text-sm font-medium">{title}</p>
          <p className="text-sm leading-relaxed text-muted-foreground">{body}</p>
        </div>
      </CardContent>
    </Card>
  );
}

function Pace({
  mode,
  label,
  reading,
  answer,
}: {
  mode: keyof typeof MINUTES_PER_TOPIC;
  label: string;
  reading: string;
  answer: string;
}) {
  return (
    <tr className="border-b border-border last:border-0">
      <td className="px-4 py-2.5 font-medium">{label}</td>
      <td className="px-4 py-2.5 text-muted-foreground">{reading}</td>
      <td className="px-4 py-2.5 text-muted-foreground">{answer}</td>
      <td className="px-4 py-2.5 text-right tabular-nums text-muted-foreground">
        ≈{MINUTES_PER_TOPIC[mode]}
      </td>
    </tr>
  );
}

function Tile({ title, body }: { title: string; body: string }) {
  return (
    <div className="rounded-4xl border border-border px-4 py-3">
      <p className="text-sm font-medium">{title}</p>
      <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{body}</p>
    </div>
  );
}
