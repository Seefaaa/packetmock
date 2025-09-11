use std::env::current_exe;

use color_eyre::Result;
use windows::Win32::{
    Foundation::VARIANT_FALSE,
    System::{
        Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
        TaskScheduler::{
            IExecAction, ILogonTrigger, ITaskService, TASK_ACTION_EXEC, TASK_CREATE_OR_UPDATE,
            TASK_LOGON_INTERACTIVE_TOKEN, TASK_RUNLEVEL_HIGHEST, TASK_TRIGGER_LOGON, TaskScheduler,
        },
        Variant::VARIANT,
    },
};
use windows_core::Interface;
use windows_registry::LOCAL_MACHINE;

use crate::{
    REGISTRY_NAME,
    service::{ServiceState, query_service},
};

pub struct Scheduler;

impl Scheduler {
    fn create_task(task_service: ITaskService) -> Result<()> {
        unsafe {
            let root_folder = task_service.GetFolder(&"\\".into())?;

            let task_defination = task_service.NewTask(0)?;

            let reg_info = task_defination.RegistrationInfo()?;
            reg_info.SetAuthor(&"Packetmock".into())?;
            reg_info.SetDescription(&"Runs the Packetmock tray icon (not the service)".into())?;

            let principal = task_defination.Principal()?;
            principal.SetLogonType(TASK_LOGON_INTERACTIVE_TOKEN)?;
            principal.SetRunLevel(TASK_RUNLEVEL_HIGHEST)?;

            let triggers = task_defination.Triggers()?;
            let trigger: ILogonTrigger = triggers.Create(TASK_TRIGGER_LOGON)?.cast()?;
            trigger.SetDelay(&"PT15S".into())?;

            let actions = task_defination.Actions()?;
            let action: IExecAction = actions.Create(TASK_ACTION_EXEC)?.cast()?;
            action.SetPath(&current_exe()?.to_string_lossy().into_owned().into())?;
            action.SetArguments(&"--task".into())?;

            let settings = task_defination.Settings()?;
            settings.SetDisallowStartIfOnBatteries(VARIANT_FALSE)?;
            settings.SetWakeToRun(VARIANT_FALSE)?;
            settings.SetExecutionTimeLimit(&"PT0S".into())?;

            root_folder.RegisterTaskDefinition(
                &"Packetmock".into(),
                &task_defination,
                TASK_CREATE_OR_UPDATE.0,
                &VARIANT::default(),
                &VARIANT::default(),
                TASK_LOGON_INTERACTIVE_TOKEN,
                &VARIANT::default(),
            )?;
        }

        Ok(())
    }
    fn create_if_not_exists() -> Result<()> {
        unsafe {
            let task_service: ITaskService =
                CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)?;
            task_service.Connect(
                &VARIANT::default(),
                &VARIANT::default(),
                &VARIANT::default(),
                &VARIANT::default(),
            )?;

            let root_folder = task_service.GetFolder(&"\\".into())?;

            match root_folder.GetTask(&"Packetmock".into()) {
                Ok(_) => Ok(()),
                Err(_) => Self::create_task(task_service),
            }
        }
    }
    pub fn create_if_should() -> Result<()> {
        let service_state = query_service()?;

        if service_state != ServiceState::NotInstalled && run_on_startup() {
            Self::create_if_not_exists()?;
        }

        Ok(())
    }
    pub fn delete() -> Result<()> {
        unsafe {
            let task_service: ITaskService =
                CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)?;
            task_service.Connect(
                &VARIANT::default(),
                &VARIANT::default(),
                &VARIANT::default(),
                &VARIANT::default(),
            )?;

            let root_folder = task_service.GetFolder(&"\\".into())?;

            let name = "Packetmock".into();

            if root_folder.GetTask(&name).is_ok() {
                root_folder.DeleteTask(&name, 0)?
            }
        }

        Ok(())
    }
}

pub fn run_on_startup() -> bool {
    let Ok(key) = LOCAL_MACHINE.open(format!("Software\\{REGISTRY_NAME}")) else {
        return true;
    };

    match key.get_u32("RunTrayOnStartup") {
        Ok(run) => run == 1,
        Err(_) => true,
    }
}

pub fn set_run_on_startup(run: bool) -> Result<()> {
    let key = LOCAL_MACHINE.create(format!("Software\\{REGISTRY_NAME}"))?;

    key.set_u32("RunTrayOnStartup", if run { 1 } else { 0 })?;

    if run {
        Scheduler::create_if_not_exists()?;
    } else {
        Scheduler::delete()?;
    }

    Ok(())
}
