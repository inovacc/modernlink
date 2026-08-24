use crate::{GitHistoryError, SelectedRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefScope {
    All,
    Head,
    Local,
    Explicit(String),
}

pub fn open_repository(path: &std::path::Path) -> Result<gix::Repository, GitHistoryError> {
    gix::discover(path).map_err(|error| GitHistoryError::Repository(error.to_string()))
}

pub fn resolve_refs(
    repository: &gix::Repository,
    scope: RefScope,
) -> Result<Vec<SelectedRef>, GitHistoryError> {
    let mut selected = match scope {
        RefScope::All => collect_references(repository, ReferenceSelection::All)?,
        RefScope::Head => collect_head(repository)?,
        RefScope::Local => collect_references(repository, ReferenceSelection::Local)?,
        RefScope::Explicit(name) => collect_explicit(repository, &name)?,
    };
    selected.sort_by(|left, right| left.name.cmp(&right.name));
    selected.dedup_by(|left, right| left.name == right.name && left.target_id == right.target_id);
    Ok(selected)
}

enum ReferenceSelection {
    All,
    Local,
}

fn collect_references(
    repository: &gix::Repository,
    selection: ReferenceSelection,
) -> Result<Vec<SelectedRef>, GitHistoryError> {
    let platform = repository
        .references()
        .map_err(|error| GitHistoryError::Reference(error.to_string()))?;
    let iterator = match selection {
        ReferenceSelection::All => platform
            .all()
            .map_err(|error| GitHistoryError::Reference(error.to_string()))?,
        ReferenceSelection::Local => platform
            .local_branches()
            .map_err(|error| GitHistoryError::Reference(error.to_string()))?,
    };

    iterator
        .map(|reference| {
            let reference =
                reference.map_err(|error| GitHistoryError::Reference(error.to_string()))?;
            selected_reference(reference)
        })
        .collect()
}

fn collect_head(repository: &gix::Repository) -> Result<Vec<SelectedRef>, GitHistoryError> {
    let head = repository
        .head_id()
        .map_err(|error| GitHistoryError::Reference(error.to_string()))?;
    Ok(vec![SelectedRef {
        name: "HEAD".to_owned(),
        target_id: head.detach().to_string(),
    }])
}

fn collect_explicit(
    repository: &gix::Repository,
    name: &str,
) -> Result<Vec<SelectedRef>, GitHistoryError> {
    let reference = repository
        .find_reference(name)
        .map_err(|error| GitHistoryError::Reference(error.to_string()))?;
    Ok(vec![selected_reference(reference)?])
}

fn selected_reference(mut reference: gix::Reference<'_>) -> Result<SelectedRef, GitHistoryError> {
    let name = reference.name().as_bstr().to_string();
    let target_id = reference
        .peel_to_id()
        .map_err(|error| GitHistoryError::Reference(error.to_string()))?
        .detach()
        .to_string();
    Ok(SelectedRef { name, target_id })
}
