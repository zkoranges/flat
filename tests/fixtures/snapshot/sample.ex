defmodule MyApp.User do
  @moduledoc """
  User module for managing user records
  """

  alias MyApp.Repo
  use Ecto.Schema

  defstruct [:name, :email]

  @doc "Creates a new user with the given attributes"
  def create(attrs) do
    %__MODULE__{}
    |> changeset(attrs)
    |> Repo.insert()
  end

  @doc "Updates an existing user"
  def update(user, attrs) do
    user
    |> changeset(attrs)
    |> Repo.update()
  end

  defp changeset(user, attrs) do
    user
    |> cast(attrs, [:name, :email])
    |> validate_required([:name, :email])
    |> validate_format(:email, ~r/@/)
  end

  defp validate_format(changeset, field, pattern) do
    case get_change(changeset, field) do
      nil -> changeset
      value -> if String.match?(value, pattern), do: changeset, else: add_error(changeset, field, "invalid")
    end
  end
end
