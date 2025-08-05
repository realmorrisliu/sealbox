"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Plus } from "lucide-react"
import { toast } from "sonner"
import { useTranslation } from "react-i18next"

interface CreateSecretDialogProps {
  children?: React.ReactNode;
}

export function CreateSecretDialog({ children }: CreateSecretDialogProps) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [newSecret, setNewSecret] = useState({
    name: "",
    value: "",
    description: "",
    ttl: "",
  })

  const handleAddSecret = () => {
    if (!newSecret.name || !newSecret.value) {
      toast.error(t('secrets.dialog.missingFields'), t('secrets.dialog.nameAndValueRequired'))
      return
    }

    // Here you would typically call your API
    toast.success(t('secrets.dialog.secretAdded'), t('secrets.dialog.hasBeenCreated', { name: newSecret.name }))
    setNewSecret({ name: "", value: "", description: "", ttl: "" })
    setOpen(false)
  }

  const handleCancel = () => {
    setNewSecret({ name: "", value: "", description: "", ttl: "" })
    setOpen(false)
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        {children}
      </DialogTrigger>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle className="text-lg">{t('secrets.dialog.addNewSecret')}</DialogTitle>
          <DialogDescription className="text-sm">
            {t('secrets.dialog.addNewSecretDescription')}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div>
            <Label htmlFor="name" className="text-xs">
              {t('secrets.dialog.name')}
            </Label>
            <Input
              id="name"
              placeholder={t('secrets.dialog.nameHelp')}
              value={newSecret.name}
              onChange={(e) => setNewSecret({ ...newSecret, name: e.target.value })}
              className="h-8"
            />
          </div>
          <div>
            <Label htmlFor="value" className="text-xs">
              {t('secrets.dialog.value')}
            </Label>
            <Textarea
              id="value"
              placeholder={t('secrets.dialog.valueHelp')}
              value={newSecret.value}
              onChange={(e) => setNewSecret({ ...newSecret, value: e.target.value })}
              className="min-h-16"
            />
          </div>
          <div>
            <Label htmlFor="description" className="text-xs">
              {t('secrets.dialog.description')}
            </Label>
            <Input
              id="description"
              placeholder={t('secrets.dialog.descriptionHelp')}
              value={newSecret.description}
              onChange={(e) => setNewSecret({ ...newSecret, description: e.target.value })}
              className="h-8"
            />
          </div>
          <div>
            <Label htmlFor="ttl" className="text-xs">
              {t('secrets.dialog.ttl')}
            </Label>
            <Input
              id="ttl"
              type="number"
              placeholder={t('secrets.dialog.ttlHelp')}
              value={newSecret.ttl}
              onChange={(e) => setNewSecret({ ...newSecret, ttl: e.target.value })}
              className="h-8"
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={handleCancel} size="sm">
            {t('common.cancel')}
          </Button>
          <Button onClick={handleAddSecret} size="sm">
            {t('secrets.controls.addSecret')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}